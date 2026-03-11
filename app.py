import os
import sys
import subprocess
import time
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
import threading

load_dotenv()
console = Console()

client = OpenAI(
    base_url="https://openrouter.ai/api/v1",
    api_key=os.getenv("OPENROUTER_API_KEY"),
)

# ── RAG imports (lazy — only fail if explicitly needed) ───────────────────────
_rag_available = False
try:
    from rag.retriever import retrieve_context
    from rag.indexer import is_indexed, get_chroma_client

    _rag_available = True
except ImportError:
    pass

# ── Pipeline logger ───────────────────────────────────────────────────────────
try:
    from pipeline.logger import PipelineLogger

    _logging_available = True
except ImportError:
    _logging_available = False

# ── Constants ─────────────────────────────────────────────────────────────────

MODEL = "arcee-ai/trinity-large-preview:free"

BASE_SYSTEM_PROMPT = """You are an expert Manim developer. Write a complete, runnable Manim script for the requested animation.
The script must define a Scene class named 'GenScene'. Use ONLY standard Manim Community Edition (v0.19+) classes and methods.
Output ONLY the python code, no markdown block or explanations. Do not use ```python``` or ``````."""

FIX_SYSTEM_PROMPT = """You are an expert Manim developer and debugger. You will be given a Manim script that failed to render, along with the error output.
Your job is to fix the code so it runs without errors. Carefully analyze the traceback and error messages.

Common issues to watch for:
- Incorrect Manim API usage (wrong class names, deprecated methods, wrong arguments)
- Syntax errors in Python code
- LaTeX rendering issues (missing escapes, bad TeX syntax)
- Import errors (using classes/functions that don't exist in Manim CE v0.19+)
- Object positioning or animation issues
- Using MathTex vs Tex incorrectly
- Forgetting to import from manim

The fixed script MUST define a Scene class named 'GenScene'.
Output ONLY the corrected Python code, no markdown blocks or explanations. Do not use ```python``` or ``````."""


# ── UI Helpers ────────────────────────────────────────────────────────────────


def print_banner(rag_active: bool = False):
    rag_status = (
        Text("   RAG: ", style="dim white")
        + Text("● ACTIVE", style="bold bright_green")
        + Text(" (version-aware Manim context)\n", style="dim white")
        if rag_active
        else Text("   RAG: ", style="dim white")
        + Text("○ INACTIVE", style="bold bright_red")
        + Text(" (run build_index.py to enable)\n", style="dim white")
    )

    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(" ✦ ", style="bold bright_magenta"),
                Text("Auto", style="bold bright_magenta"),
                Text("Manim", style="bold bright_cyan"),
                Text("  AI-powered Manim animation generator\n", style="dim white"),
                Text(
                    "   v0.2.0  •  OpenRouter  •  arcee-ai/trinity\n",
                    style="dim bright_black",
                ),
                Text(
                    "   Self-healing: continuous auto-fix loop until success\n",
                    style="dim bright_yellow",
                ),
                rag_status,
            ),
            box=box.ROUNDED,
            border_style="bright_magenta",
            padding=(0, 2),
        )
    )
    console.print()


def print_step(icon: str, label: str, value: str = "", style: str = "bright_cyan"):
    line = Text()
    line.append(f" {icon} ", style=f"bold {style}")
    line.append(label, style="bold white")
    if value:
        line.append(f"  {value}", style="dim white")
    console.print(line)


def print_success(message: str):
    text = Text()
    text.append(" ✓ ", style="bold bright_green")
    text.append(message, style="bold white")
    console.print(text)


def print_error(message: str):
    console.print(
        Panel(
            Text.assemble(
                Text(" ✗  Error\n", style="bold bright_red"),
                Text(f" {message}", style="white"),
            ),
            border_style="bright_red",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_rag_status(raw_results: list[dict]):
    """Show a compact summary of what RAG retrieved."""
    if not raw_results:
        return
    by_type: dict[str, int] = {}
    for r in raw_results:
        t = r.get("source_type", "?")
        by_type[t] = by_type.get(t, 0) + 1
    sims = [r.get("similarity", 0) for r in raw_results]
    avg_sim = sum(sims) / len(sims) if sims else 0

    parts = []
    for t, count in sorted(by_type.items()):
        parts.append(f"{count}×{t}")

    console.print(
        Panel(
            Text.assemble(
                Text(" 🔍 ", style="bold bright_cyan"),
                Text("RAG Retrieved", style="bold bright_cyan"),
                Text(
                    f"  {len(raw_results)} chunks  [ {' | '.join(parts)} ]\n",
                    style="bold white",
                ),
                Text(f"   avg similarity: {avg_sim:.3f}  ", style="dim white"),
                Text(
                    f"max: {max(sims):.3f}  min: {min(sims):.3f}",
                    style="dim bright_black",
                ),
            ),
            border_style="bright_cyan",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_heal_attempt(attempt: int):
    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(" 🔧 ", style="bold bright_yellow"),
                Text("Self-Healing", style="bold bright_yellow"),
                Text(f"  attempt {attempt}\n", style="bold white"),
                Text("   Analyzing error and regenerating code…", style="dim white"),
            ),
            border_style="bright_yellow",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_heal_success(attempt: int):
    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(" ✦ ", style="bold bright_green"),
                Text("Self-Healed!", style="bold bright_green"),
                Text(f"  fixed on attempt {attempt}\n", style="bold white"),
                Text(
                    "   The code was automatically repaired and rendered successfully.",
                    style="dim white",
                ),
            ),
            border_style="bright_green",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_error_summary(error_output: str):
    lines = error_output.strip().splitlines()
    error_lines = []
    for line in reversed(lines):
        error_lines.insert(0, line)
        if line.strip().startswith("Traceback") or line.strip().startswith("File"):
            pass
        if len(error_lines) >= 15:
            break
    summary = "\n".join(error_lines[-20:])
    syntax = Syntax(
        summary, "pytb", theme="monokai", word_wrap=True, line_numbers=False
    )
    console.print(
        Panel(
            syntax,
            title="[bold bright_red] Error Output [/bold bright_red]",
            title_align="left",
            border_style="bright_red",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


def print_code_preview(code: str, filename: str):
    lines = code.splitlines()
    preview_lines = lines[:20]
    truncated = len(lines) > 20
    preview = "\n".join(preview_lines)
    if truncated:
        preview += f"\n  … ({len(lines) - 20} more lines)"
    syntax = Syntax(
        preview, "python", theme="monokai", line_numbers=True, word_wrap=False
    )
    console.print(
        Panel(
            syntax,
            title=f"[bold bright_black] {filename} [/bold bright_black]",
            title_align="left",
            border_style="bright_black",
            box=box.ROUNDED,
            padding=(0, 0),
        )
    )


def print_pipeline_summary(logger: "PipelineLogger | None"):
    """Print a final pipeline summary after the session."""
    if logger is None:
        return
    s = logger.summary_dict()
    outcome_style = (
        "bold bright_green" if s["outcome"] == "success" else "bold bright_red"
    )
    outcome_icon = "✦" if s["outcome"] == "success" else "✗"
    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(f" {outcome_icon} ", style=outcome_style),
                Text("Pipeline Summary", style="bold white"),
                Text(f"  [{s['outcome'].upper()}]\n", style=outcome_style),
                Text(f"   Total attempts : {s['attempts']}\n", style="dim white"),
                Text(f"   RAG chunks used: {s['rag_chunks']}\n", style="dim white"),
                Text(f"   Total time     : {s['duration_s']}s\n", style="dim white"),
                Text(
                    f"   Log saved      : logs/pipeline_log.jsonl",
                    style="dim bright_black",
                ),
            ),
            border_style="bright_magenta",
            box=box.ROUNDED,
            padding=(0, 1),
        )
    )


# ── Core Logic ────────────────────────────────────────────────────────────────


def _llm_call(
    messages: list[dict], spinner_label: str = "Thinking"
) -> tuple[str, float]:
    """Call the LLM with animated spinner. Returns (content, latency_s)."""
    start = time.time()
    result = {"code": None, "error": None}

    def api_call():
        try:
            response = client.chat.completions.create(model=MODEL, messages=messages)
            result["code"] = response.choices[0].message.content or ""
        except Exception as e:
            result["error"] = e

    thread = threading.Thread(target=api_call)
    thread.start()

    frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    frame_idx = 0

    with Live(console=console, refresh_per_second=10, transient=True) as live:
        while thread.is_alive():
            elapsed = time.time() - start
            frame = frames[frame_idx % len(frames)]
            frame_idx += 1
            live.update(
                Text.assemble(
                    Text(f" {frame} ", style="bold bright_magenta"),
                    Text(spinner_label, style="bold white"),
                    Text(
                        f"  generating Manim code… ({elapsed:.1f}s)", style="dim white"
                    ),
                )
            )
            time.sleep(0.1)

    thread.join()
    latency = time.time() - start

    if result["error"]:
        raise result["error"]

    return result["code"], latency


def build_system_prompt(rag_context: str) -> str:
    """Compose system prompt with optional RAG context appended."""
    if not rag_context:
        return BASE_SYSTEM_PROMPT
    return (
        BASE_SYSTEM_PROMPT
        + "\n\n"
        + "Use the following Manim API reference when writing code. "
        + "Prefer classes and methods shown here, as they are confirmed to exist "
        + "in the installed version of Manim.\n\n"
        + rag_context
    )


def generate_code(prompt: str, rag_context: str = "") -> tuple[str, float]:
    """Call the LLM and return (generated_code, latency_seconds)."""
    console.print()
    messages = [
        {"role": "system", "content": build_system_prompt(rag_context)},
        {"role": "user", "content": prompt},
    ]
    code, latency = _llm_call(messages, spinner_label="Thinking")
    print_step("✓", "Code generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


def fix_code(
    original_code: str, error_output: str, original_prompt: str, rag_context: str = ""
) -> tuple[str, float]:
    """Send the broken code and error back to the LLM for a fix."""
    console.print()
    fix_prompt = f"""The following Manim script was generated for this request:
---
{original_prompt}
---

Here is the code that failed:
```python
{original_code}
```

Here is the error output from running it:
```
{error_output}
```

Please fix the code so it runs without errors. Output ONLY the corrected Python code."""

    system = FIX_SYSTEM_PROMPT
    if rag_context:
        system += (
            "\n\nUse this Manim API reference to correct class/method names:\n\n"
            + rag_context
        )

    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": fix_prompt},
    ]
    code, latency = _llm_call(messages, spinner_label="Healing")
    print_step("✓", "Fix generated", f"({latency:.1f}s)", style="bright_green")
    return code, latency


def render_scene(script_path: Path) -> tuple[int, str]:
    """Run manim to render the scene. Returns (returncode, combined_output)."""
    console.print()
    console.print(Rule(style="bright_black"))
    console.print(
        Text.assemble(
            Text(" ▶ ", style="bold bright_yellow"),
            Text("Rendering scene", style="bold white"),
            Text("  manim -ql GenScene", style="dim bright_black"),
        )
    )
    console.print(Rule(style="bright_black"))
    console.print()

    result = subprocess.run(
        [sys.executable, "-m", "manim", "-ql", str(script_path), "GenScene"],
        capture_output=True,
        text=True,
    )

    if result.stdout.strip():
        for line in result.stdout.strip().splitlines():
            console.print(Text(f"  {line}", style="dim white"))

    console.print()
    console.print(Rule(style="bright_black"))

    if result.returncode == 0:
        print_success("Render complete!")
    else:
        print_error(f"Manim exited with code {result.returncode}")

    full_output = ""
    if result.stderr:
        full_output += result.stderr
    if result.returncode != 0 and result.stdout:
        full_output += "\n" + result.stdout

    return result.returncode, full_output.strip()


def clean_code(code: str) -> str:
    """Strip markdown fences and whitespace from LLM output."""
    code = code.strip()
    if code.startswith("```python"):
        code = code[len("```python") :]
    elif code.startswith("```"):
        code = code[len("```") :]
    if code.endswith("```"):
        code = code[:-3]
    return code.strip()


def validate_code_syntax(code: str) -> tuple[bool, str]:
    try:
        compile(code, "<generated_scene>", "exec")
        return True, ""
    except SyntaxError as e:
        return False, f"SyntaxError at line {e.lineno}: {e.msg}"


def validate_scene_class(code: str) -> tuple[bool, str]:
    if "class GenScene" not in code:
        return False, "Generated code does not contain a 'class GenScene' definition."
    return True, ""


# ── Self-Healing Loop ─────────────────────────────────────────────────────────


def self_healing_loop(
    code: str,
    prompt: str,
    output_file: Path,
    rag_context: str = "",
    logger: "PipelineLogger | None" = None,
) -> bool:
    """
    Attempt to render the scene. If it fails, send the error + RAG context
    back to the LLM. Repeat indefinitely until it works.

    Returns True when the scene is eventually rendered successfully.
    """
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
                "◆",
                "Updated" if is_heal else "Saved to",
                str(output_file.absolute()),
                style="bright_cyan",
            )
            console.print()
            print_code_preview(current_code, output_file.name)
            console.print()

            t_render_start = time.time()
            returncode, error_output = render_scene(output_file)
            latency = time.time() - t_render_start

        render_success = returncode == 0

        if logger:
            kwargs = {
                "attempt": attempt,
                "code": current_code,
                "syntax_ok": syntax_ok,
                "scene_class_ok": scene_ok,
                "render_success": render_success,
                "error_output": error_output,
                "is_heal_attempt": is_heal,
            }
            if latency > 0:
                kwargs["latency_s"] = latency
            logger.log_generation(**kwargs)

        if render_success:
            if is_heal:
                print_heal_success(attempt)
            return True

        if error_output:
            print_error_summary(error_output)

            # Query RAG with the specific error to get more relevant fix documentation
            error_lines = error_output.strip().splitlines()
            last_few_errors = "\n".join(error_lines[-5:])  # Get the actual exception
            error_query = f"Fix Manim error: {last_few_errors}"
            error_rag_context = get_rag_context(
                error_query, logger, step_label="Querying error context…"
            )

            if error_rag_context:
                rag_context = error_rag_context  # Replace general context with specific error context

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


def get_rag_context(
    query: str,
    logger: "PipelineLogger | None",
    step_label: str = "Retrieving Manim context",
) -> str:
    """Retrieve RAG context if index is available. Returns empty string otherwise."""
    if not _rag_available:
        return ""
    try:
        chroma_client = get_chroma_client()
        if not is_indexed(chroma_client):
            if step_label == "Retrieving Manim context":  # Only warn on the first query
                console.print(
                    " [dim yellow]⚠ RAG index not found. "
                    "Run[/dim yellow] [bold]uv run python build_index.py[/bold] "
                    "[dim yellow]to enable version-aware generation.[/dim yellow]"
                )
                console.print()
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
    # Check RAG index status for banner
    rag_active = False
    if _rag_available:
        try:
            rag_active = is_indexed(get_chroma_client())
        except Exception:
            pass

    print_banner(rag_active=rag_active)

    # Get prompt
    if len(sys.argv) > 1:
        prompt = " ".join(sys.argv[1:])
        print_step("◆", "Prompt", prompt, style="bright_cyan")
    else:
        console.print(
            Text.assemble(
                Text(" ◆ ", style="bold bright_cyan"),
                Text("What animation would you like to create?", style="bold white"),
            )
        )
        console.print()
        prompt = Prompt.ask(
            "   [dim]Describe your animation[/dim]",
            console=console,
        ).strip()

    console.print()
    if not prompt:
        print_error("No prompt provided.")
        sys.exit(1)

    # Initialise pipeline logger
    logger = PipelineLogger(prompt) if _logging_available else None

    # ── RAG retrieval ─────────────────────────────────────────────────────────
    rag_context = get_rag_context(prompt, logger)

    # ── Code generation ───────────────────────────────────────────────────────
    try:
        t_gen_start = time.time()
        code, gen_latency = generate_code(prompt, rag_context)
    except Exception as e:
        print_error(str(e))
        if logger:
            logger.finalize(success=False)
        sys.exit(1)

    code = clean_code(code)
    if not code:
        print_error("The model returned empty output. Try a different prompt.")
        if logger:
            logger.finalize(success=False)
        sys.exit(1)

    # ── Self-healing render loop ──────────────────────────────────────────────
    output_file = Path("generated_scene.py")
    success = self_healing_loop(code, prompt, output_file, rag_context, logger)

    # ── Finalise ──────────────────────────────────────────────────────────────
    if logger:
        logger.finalize(success=success)
        print_pipeline_summary(logger)

    console.print()
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
