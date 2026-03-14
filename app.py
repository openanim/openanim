import os, sys, subprocess, time, threading
from pathlib import Path
from dotenv import load_dotenv
from openai import OpenAI
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.syntax import Syntax
from rich.live import Live
from rich.rule import Rule
from rich import box
from rich.prompt import Prompt

load_dotenv()
console = Console()

client = OpenAI(
    base_url="https://openrouter.ai/api/v1", api_key=os.getenv("OPENROUTER_API_KEY")
)

_rag_available = False
try:
    from rag.retriever import retrieve_context
    from rag.indexer import is_indexed, get_chroma_client

    _rag_available = True
except ImportError:
    pass

try:
    from pipeline.logger import PipelineLogger

    _logging_available = True
except ImportError:
    _logging_available = False

MODEL = "openrouter/auto"

BASE_SYSTEM_PROMPT = """You are an expert Manim developer. Write a complete, runnable Manim script for the requested animation.
The script must define a Scene class named 'GenScene'. Use ONLY standard Manim Community Edition (v0.19+) classes and methods.
Output ONLY the python code, no markdown block or explanations. Do not use ```python``` or ``````."""

FIX_SYSTEM_PROMPT = """You are an expert Manim developer and debugger. Fix the provided Manim script that failed to render.
Analyze the traceback carefully for: wrong API usage, syntax errors, LaTeX issues, missing imports, or bad animations.
The fixed script MUST define a Scene class named 'GenScene'.
Output ONLY the corrected Python code, no markdown blocks or explanations."""


# ── UI Helpers ────────────────────────────────────────────────────────────────


def _panel(content, border: str = "bright_magenta"):
    console.print(Panel(content, box=box.ROUNDED, border_style=border, padding=(0, 1)))


def print_banner(rag_active=False):
    rag_text = (
        Text("● ACTIVE", style="bold bright_green")
        if rag_active
        else Text("○ INACTIVE (run build_index.py to enable)", style="bold bright_red")
    )
    _panel(
        Text.assemble(
            Text(" ✦ ", style="bold bright_magenta"),
            Text("AutoManim", style="bold bright_cyan"),
            Text("  AI-powered Manim animation generator\n   RAG: ", style="dim white"),
            rag_text,
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
            Text(" ✓ ", style="bold bright_green"), Text(msg, style="bold white")
        )
    )


def print_error(msg):
    _panel(
        Text.assemble(
            Text(" ✗  Error\n", style="bold bright_red"), Text(f" {msg}", style="white")
        ),
        "bright_red",
    )


def print_rag_status(raw_results):
    if not raw_results:
        return
    by_type = {}
    for r in raw_results:
        by_type[r.get("source_type", "?")] = (
            by_type.get(r.get("source_type", "?"), 0) + 1
        )
    sims = [r.get("similarity", 0) for r in raw_results]
    parts = " | ".join(f"{c}×{t}" for t, c in sorted(by_type.items()))
    _panel(
        Text.assemble(
            Text(
                f" 🔍 RAG Retrieved  {len(raw_results)} chunks  [{parts}]\n",
                style="bold bright_cyan",
            ),
            Text(
                f"   avg={sum(sims)/len(sims):.3f}  max={max(sims):.3f}  min={min(sims):.3f}",
                style="dim white",
            ),
        ),
        "bright_cyan",
    )


def print_heal_attempt(attempt):
    _panel(
        Text.assemble(
            Text(f" 🔧 Self-Healing  attempt {attempt}\n", style="bold bright_yellow"),
            Text("   Analyzing error and regenerating code…", style="dim white"),
        ),
        "bright_yellow",
    )


def print_heal_success(attempt):
    _panel(
        Text.assemble(
            Text(
                f" ✦ Self-Healed!  fixed on attempt {attempt}\n",
                style="bold bright_green",
            ),
            Text(
                "   Automatically repaired and rendered successfully.",
                style="dim white",
            ),
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
        f"\n  … ({len(lines)-20} more lines)" if len(lines) > 20 else ""
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


def print_pipeline_summary(logger):
    if not logger:
        return
    s = logger.summary_dict()
    style = "bold bright_green" if s["outcome"] == "success" else "bold bright_red"
    icon = "✦" if s["outcome"] == "success" else "✗"
    _panel(
        Text.assemble(
            Text(f" {icon} Pipeline Summary  [{s['outcome'].upper()}]\n", style=style),
            Text(
                f"   Attempts: {s['attempts']}  RAG chunks: {s['rag_chunks']}  Time: {s['duration_s']}s\n",
                style="dim white",
            ),
            Text("   Log: logs/pipeline_log.jsonl", style="dim bright_black"),
        )
    )


# ── Core Logic ────────────────────────────────────────────────────────────────


def _llm_call(messages, spinner_label="Thinking"):
    start = time.time()
    result = {"code": None, "error": None}

    def api_call():
        try:
            result["code"] = (
                client.chat.completions.create(model=MODEL, messages=messages)
                .choices[0]
                .message.content
                or ""
            )
        except Exception as e:
            result["error"] = e

    thread = threading.Thread(target=api_call)
    thread.start()
    frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    with Live(console=console, refresh_per_second=10, transient=True) as live:
        for i in range(10**9):
            if not thread.is_alive():
                break
            live.update(
                Text.assemble(
                    Text(f" {frames[i%10]} ", style="bold bright_magenta"),
                    Text(spinner_label, style="bold white"),
                    Text(
                        f"  generating… ({time.time()-start:.1f}s)", style="dim white"
                    ),
                )
            )
            time.sleep(0.1)
    thread.join()
    if result["error"]:
        raise result["error"]
    return result["code"], time.time() - start


def build_system_prompt(rag_context):
    if not rag_context:
        return BASE_SYSTEM_PROMPT
    return (
        BASE_SYSTEM_PROMPT + "\n\nUse the following Manim API reference. "
        "Prefer classes and methods shown here as they exist in the installed version.\n\n"
        + rag_context
    )


def generate_code(prompt, rag_context=""):
    console.print()
    code, latency = _llm_call(
        [
            {"role": "system", "content": build_system_prompt(rag_context)},
            {"role": "user", "content": prompt},
        ],
        spinner_label="Thinking",
    )
    print_step("✓", "Code generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


def fix_code(original_code, error_output, original_prompt, rag_context=""):
    console.print()
    system = FIX_SYSTEM_PROMPT
    if rag_context:
        system += (
            "\n\nUse this Manim API reference to correct class/method names:\n\n"
            + rag_context
        )
    fix_prompt = (
        f"Request:\n---\n{original_prompt}\n---\n\n"
        f"Failed code:\n```python\n{original_code}\n```\n\n"
        f"Error:\n```\n{error_output}\n```\n\nFix it. Output ONLY corrected Python."
    )
    code, latency = _llm_call(
        [
            {"role": "system", "content": system},
            {"role": "user", "content": fix_prompt},
        ],
        spinner_label="Healing",
    )
    print_step("✓", "Fix generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


def render_scene(script_path):
    console.print(Rule(style="bright_black"))
    console.print(
        Text.assemble(
            Text(" ▶ Rendering scene", style="bold white"),
            Text("  manim -ql GenScene", style="dim bright_black"),
        )
    )
    console.print(Rule(style="bright_black"))
    result = subprocess.run(
        [sys.executable, "-m", "manim", "-ql", str(script_path), "GenScene"],
        capture_output=True,
        text=True,
    )
    for line in result.stdout.strip().splitlines():
        console.print(Text(f"  {line}", style="dim white"))
    console.print(Rule(style="bright_black"))
    if result.returncode == 0:
        print_success("Render complete!")
    else:
        print_error(f"Manim exited with code {result.returncode}")
    full_output = result.stderr + (
        "\n" + result.stdout if result.returncode != 0 else ""
    )
    return result.returncode, full_output.strip()


def clean_code(code):
    code = code.strip()
    for fence in ("```python", "```"):
        if code.startswith(fence):
            code = code[len(fence) :]
    if code.endswith("```"):
        code = code[:-3]
    return code.strip()


def validate_code_syntax(code):
    try:
        compile(code, "<generated_scene>", "exec")
        return True, ""
    except SyntaxError as e:
        return False, f"SyntaxError at line {e.lineno}: {e.msg}"


def validate_scene_class(code):
    ok = "class GenScene" in code
    return ok, (
        "" if ok else "Generated code does not contain a 'class GenScene' definition."
    )


# ── Self-Healing Loop ─────────────────────────────────────────────────────────


def self_healing_loop(code, prompt, output_file, rag_context="", logger=None):
    current_code, attempt = code, 0
    while True:
        is_heal = attempt > 0
        syntax_ok, syntax_err = validate_code_syntax(current_code)
        scene_ok, scene_err = validate_scene_class(current_code)
        error_output, returncode, latency = "", -1, 0.0

        if not syntax_ok:
            error_output = syntax_err
            print_error(
                f"{'Fixed' if is_heal else 'Generated'} code has a syntax error: {syntax_err}"
            )
        elif not scene_ok:
            error_output = scene_err
            print_error(scene_err)
        else:
            output_file.write_text(current_code, encoding="utf-8")
            print_step(
                "◆", "Updated" if is_heal else "Saved to", str(output_file.absolute())
            )
            console.print()
            print_code_preview(current_code, output_file.name)
            console.print()
            t0 = time.time()
            returncode, error_output = render_scene(output_file)
            latency = time.time() - t0

        render_success = returncode == 0
        if logger:
            logger.log_generation(
                attempt=attempt,
                code=current_code,
                syntax_ok=syntax_ok,
                scene_class_ok=scene_ok,
                render_success=render_success,
                error_output=error_output,
                is_heal_attempt=is_heal,
                **({"latency_s": latency} if latency > 0 else {}),
            )

        if render_success:
            if is_heal:
                print_heal_success(attempt)
            return True

        if error_output:
            print_error_summary(error_output)
            last_few = "\n".join(error_output.strip().splitlines()[-5:])
            err_rag = get_rag_context(
                f"Fix Manim error: {last_few}", logger, "Querying error context…"
            )
            if err_rag:
                rag_context = err_rag

        attempt += 1
        print_heal_attempt(attempt)
        try:
            current_code, _ = fix_code(current_code, error_output, prompt, rag_context)
            current_code = clean_code(current_code)
            if not current_code:
                print_error("The model returned empty fix output.")
                return False
        except Exception as e:
            print_error(f"Failed to call LLM for fix: {e}")
            return False


# ── RAG Orchestration ─────────────────────────────────────────────────────────


def get_rag_context(query, logger, step_label="Retrieving Manim context"):
    if not _rag_available:
        return ""
    try:
        chroma_client = get_chroma_client()
        if not is_indexed(chroma_client):
            if step_label == "Retrieving Manim context":
                console.print(
                    " [dim yellow]⚠ RAG index not found. Run[/dim yellow] [bold]uv run python build_index.py[/bold]"
                )
            return ""
        print_step("🔍", "RAG Query", step_label, style="bright_cyan")
        rag_context, raw_results = retrieve_context(query, client=chroma_client)
        print_rag_status(raw_results)
        if logger and raw_results:
            logger.log_rag_retrieval(raw_results, len(rag_context))
        return rag_context
    except Exception as e:
        console.print(f" [dim red]RAG retrieval failed: {e}[/dim red]")
        return ""


# ── Main ──────────────────────────────────────────────────────────────────────


def main():
    rag_active = False
    if _rag_available:
        try:
            rag_active = is_indexed(get_chroma_client())
        except Exception:
            pass
    print_banner(rag_active)

    prompt = (
        " ".join(sys.argv[1:])
        if len(sys.argv) > 1
        else Prompt.ask(
            "   [dim]Describe your animation[/dim]", console=console
        ).strip()
    )
    if not prompt:
        print_error("No prompt provided.")
        sys.exit(1)

    logger = PipelineLogger(prompt) if _logging_available else None
    rag_context = get_rag_context(prompt, logger)

    try:
        code, _ = generate_code(prompt, rag_context)
    except Exception as e:
        print_error(str(e))
        if logger:
            logger.finalize(success=False)
        sys.exit(1)

    code = clean_code(code)
    if not code:
        print_error("The model returned empty output.")
        if logger:
            logger.finalize(success=False)
        sys.exit(1)

    success = self_healing_loop(
        code, prompt, Path("generated_scene.py"), rag_context, logger
    )

    if logger:
        logger.finalize(success=success)
        print_pipeline_summary(logger)
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
