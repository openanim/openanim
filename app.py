import argparse
import os
import sys
import subprocess
import time
import threading
from pathlib import Path

from dotenv import load_dotenv
from openai import OpenAI
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.syntax import Syntax
from rich.live import Live
from rich import box
from rich.prompt import Prompt

from backend import get_cfg, get_backend, list_backends

load_dotenv()
console = Console()

client = OpenAI(
    base_url=get_cfg("openrouter.base_url", "https://openrouter.ai/api/v1"),
    api_key=os.getenv("OPENROUTER_API_KEY"),
)

MODEL = get_cfg("llm.model", "openrouter/auto")


# ── UI Helpers ────────────────────────────────────────────────────────────────


def _panel(content, border: str = "bright_magenta"):
    console.print(Panel(content, box=box.ROUNDED, border_style=border, padding=(0, 1)))


def print_banner(backend):
    _panel(
        Text.assemble(
            Text(" > ", style="bold bright_magenta"),
            Text("OpenAnim", style="bold bright_cyan"),
            Text("  AI-powered animation generator\n", style="dim white"),
            Text(f"   Backend: ", style="dim white"),
            Text(backend.name.upper(), style="bold bright_green"),
        )
    )


def print_step(icon, label, value="", style="bright_cyan"):
    line = Text()
    line.append(f" {icon} ", style=f"bold {style}")
    line.append(label, style="bold white")
    if value:
        line.append(f"  {value}", style="dim white")
    console.print(line)


def print_success(msg):
    console.print(
        Text.assemble(
            Text(" [OK] ", style="bold bright_green"), Text(msg, style="bold white")
        )
    )


def print_error(msg):
    _panel(
        Text.assemble(
            Text(" ERR  Error\n", style="bold bright_red"), Text(f" {msg}", style="white")
        ),
        "bright_red",
    )


def print_heal_attempt(attempt):
    _panel(
        Text.assemble(
            Text(f" [FIX] Self-Healing  attempt {attempt}\n", style="bold bright_yellow"),
            Text("   Analyzing error and regenerating code...", style="dim white"),
        ),
        "bright_yellow",
    )


def print_heal_success(attempt):
    _panel(
        Text.assemble(
            Text(
                f" > Self-Healed!  fixed on attempt {attempt}\n",
                style="bold bright_green",
            ),
            Text("   Automatically repaired and rendered successfully.", style="dim white"),
        ),
        "bright_green",
    )


def print_error_summary(error_output):
    summary = "\n".join(error_output.strip().splitlines()[-20:])
    console.print(
        Panel(
            Syntax(summary, "pytb", theme="monokai", word_wrap=True),
            title="[bold bright_red] Error Output [/bold bright_red]",
            title_align="left",
            border_style="bright_red",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_code_preview(code, filename):
    lines = code.splitlines()
    preview = "\n".join(lines[:20]) + (
        f"\n  ... ({len(lines) - 20} more lines)" if len(lines) > 20 else ""
    )
    console.print(
        Panel(
            Syntax(preview, "python", theme="monokai", line_numbers=True),
            title=f"[bold bright_black] {filename} [/bold bright_black]",
            title_align="left",
            border_style="bright_black",
            box=box.ROUNDED,
        )
    )


def print_render_header(backend, quality):
    console.print(Text("-" * 60, style="dim bright_black"))
    console.print(
        Text.assemble(
            Text(" [>] Rendering scene", style="bold white"),
            Text(f"  {backend.name} -q{quality}", style="dim bright_black"),
        )
    )
    console.print(Text("-" * 60, style="dim bright_black"))


def print_render_output(stdout):
    for line in stdout.strip().splitlines():
        console.print(Text(f"  {line}", style="dim white"))
    console.print(Text("-" * 60, style="dim bright_black"))


# ── Core Logic ────────────────────────────────────────────────────────────────


def _llm_call(messages, spinner_label="Thinking"):
    start = time.time()
    result = {"code": None, "error": None}

    def api_call():
        try:
            resp = client.chat.completions.create(model=MODEL, messages=messages)
            if resp.choices:
                result["code"] = resp.choices[0].message.content or ""
            else:
                result["error"] = RuntimeError("LLM returned empty choices (rate limit / no free model available)")
        except Exception as e:
            result["error"] = e

    thread = threading.Thread(target=api_call)
    thread.start()
    frames = ["|", "/", "-", "\\"]
    with Live(console=console, refresh_per_second=10, transient=True) as live:
        for i in range(10**9):
            if not thread.is_alive():
                break
            live.update(
                Text.assemble(
                    Text(f" {frames[i % 4]} ", style="bold bright_magenta"),
                    Text(spinner_label, style="bold white"),
                    Text(f"  generating... ({time.time() - start:.1f}s)", style="dim white"),
                )
            )
            time.sleep(0.1)
    thread.join()
    if result["error"]:
        raise result["error"]
    return result["code"], time.time() - start


def clean_code(code: str) -> str:
    code = code.strip()
    for fence in ("```python", "```"):
        if code.startswith(fence):
            code = code[len(fence) :]
    if code.endswith("```"):
        code = code[:-3]
    return code.strip()


def generate_code(prompt: str, backend) -> tuple[str, float]:
    console.print()
    messages = [
        {"role": "system", "content": backend.system_prompt()},
        {"role": "user", "content": prompt},
    ]
    code, latency = _llm_call(messages, spinner_label="Thinking")
    print_step("[OK]", "Code generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


def fix_code(original_code: str, error_output: str, original_prompt: str, backend) -> tuple[str, float]:
    console.print()
    system = backend.fix_system_prompt()
    fix_prompt = (
        f"Request:\n---\n{original_prompt}\n---\n\n"
        f"Failed code:\n```python\n{original_code}\n```\n\n"
        f"Error:\n```\n{error_output}\n```\n\nFix it. Output ONLY corrected Python."
    )
    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": fix_prompt},
    ]
    code, latency = _llm_call(messages, spinner_label="Healing")
    print_step("[OK]", "Fix generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


# ── Self-Healing Loop ─────────────────────────────────────────────────────────


def self_healing_loop(code: str, prompt: str, backend, quality: str, max_attempts: int) -> bool:
    current_code, attempt = code, 0
    while attempt <= max_attempts:
        is_heal = attempt > 0
        output_file = backend.get_output_path()

        valid, err = backend.validate(current_code)
        if not valid:
            print_error(f"{'Fixed' if is_heal else 'Generated'} code invalid: {err}")
        else:
            output_file.write_text(current_code, encoding="utf-8")
            print_step(">", "Updated" if is_heal else "Saved to", str(output_file.absolute()))
            console.print()
            print_code_preview(current_code, output_file.name)
            console.print()

            print_render_header(backend, quality)
            exit_code, output = backend.render(output_file, quality)
            print_render_output(output if exit_code == 0 else "")

            if exit_code == 0:
                print_success("Render complete!")
                if is_heal:
                    print_heal_success(attempt)
                return True
            else:
                print_error(f"{backend.name} exited with code {exit_code}")
                print_error_summary(output)
                if attempt >= max_attempts:
                    print_error(f"Max healing attempts ({max_attempts}) reached.")
                    return False

        attempt += 1
        if attempt > max_attempts:
            break
        print_heal_attempt(attempt)
        try:
            current_code, _ = fix_code(current_code, output if "output" in dir() and output else err, prompt, backend)
            current_code = clean_code(current_code)
            if not current_code:
                print_error("The model returned empty fix output.")
                return False
        except Exception as e:
            print_error(f"Failed to call LLM for fix: {e}")
            return False

    return False


# ── CLI ───────────────────────────────────────────────────────────────────────


def parse_args():
    parser = argparse.ArgumentParser(description="OpenAnim — AI-powered animation generator")
    parser.add_argument("prompt", nargs="*", help="Natural language description of the animation")
    parser.add_argument(
        "-b", "--backend",
        choices=list_backends(),
        default="manim",
        help="Rendering backend (default: manim)",
    )
    parser.add_argument(
        "-q", "--quality",
        choices=["l", "m", "h", "p", "k"],
        default=None,
        help=f"Render quality (default: {get_cfg('manim.quality', 'l')})",
    )
    parser.add_argument(
        "--max-heal",
        type=int,
        default=None,
        help=f"Max self-healing attempts (default: {get_cfg('healing.max_attempts', 3)})",
    )
    return parser.parse_args()


# ── Main ──────────────────────────────────────────────────────────────────────


def main():
    args = parse_args()
    backend = get_backend(args.backend)
    quality = args.quality or get_cfg("manim.quality", "l")
    max_heal = args.max_heal if args.max_heal is not None else get_cfg("healing.max_attempts", 3)

    print_banner(backend)

    prompt = (
        " ".join(args.prompt)
        if args.prompt
        else Prompt.ask("   [dim]Describe your animation[/dim]", console=console).strip()
    )
    if not prompt:
        print_error("No prompt provided.")
        sys.exit(1)

    try:
        code, _ = generate_code(prompt, backend)
    except Exception as e:
        print_error(str(e))
        sys.exit(1)

    code = clean_code(code)
    if not code:
        print_error("The model returned empty output.")
        sys.exit(1)

    success = self_healing_loop(code, prompt, backend, quality, max_heal)
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
