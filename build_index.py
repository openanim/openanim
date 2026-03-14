import sys, time
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich import box

console = Console()


def main():
    force = "--force" in sys.argv
    console.print(
        Panel(
            Text.assemble(
                Text(" ◆ AutoManim RAG Index Builder\n", style="bold white"),
                Text(
                    f"   Mode: {'FORCE REBUILD' if force else 'incremental'}",
                    style="dim bright_yellow",
                ),
            ),
            box=box.ROUNDED,
            border_style="bright_magenta",
            padding=(0, 2),
        )
    )

    try:
        from rag.indexer import build_index, get_chroma_client
    except ImportError as e:
        console.print(f"[bold red]Import error:[/bold red] {e}")
        sys.exit(1)

    get_chroma_client()
    start = time.time()
    try:
        build_index(force=force)
    except FileNotFoundError as e:
        console.print(f"[bold red]✗ Error:[/bold red] {e}")
        sys.exit(1)
    except Exception as e:
        console.print(f"[bold red]✗ Unexpected error:[/bold red] {e}")
        raise

    console.print(
        Panel(
            Text.assemble(
                Text(" ✦ Indexing complete!\n", style="bold bright_green"),
                Text(f"   Time: {time.time()-start:.1f}s  →  ", style="dim white"),
                Text("uv run python app.py", style="bold bright_cyan"),
            ),
            box=box.ROUNDED,
            border_style="bright_green",
            padding=(0, 2),
        )
    )


if __name__ == "__main__":
    main()
