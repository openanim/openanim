"""
build_index.py
--------------
One-time (or on-update) script to index the Manim source corpus into ChromaDB.

Run this before using AutoManim:
    uv run python build_index.py

Add --force to rebuild from scratch:
    uv run python build_index.py --force

The index is stored in ./chroma_db/ and is reused across all app.py runs.
"""

import sys
import time
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich import box

console = Console()


def main():
    force = "--force" in sys.argv

    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(" ◆ ", style="bold bright_magenta"),
                Text("AutoManim RAG Index Builder\n", style="bold white"),
                Text("   Indexing Manim source, docs, changelogs & examples\n", style="dim white"),
                Text(f"   Mode: {'FORCE REBUILD' if force else 'incremental (skip if exists)'}", style="dim bright_yellow"),
            ),
            box=box.ROUNDED,
            border_style="bright_magenta",
            padding=(0, 2),
        )
    )
    console.print()

    try:
        from rag.indexer import build_index, is_indexed, get_chroma_client
    except ImportError as e:
        console.print(f"[bold red]Import error:[/bold red] {e}")
        console.print("[dim]Make sure you are running from the AutoManim project root.[/dim]")
        sys.exit(1)

    client = get_chroma_client()



    start = time.time()
    console.print(" [bold bright_cyan]▶[/bold bright_cyan] Starting indexing pipeline…\n")

    try:
        build_index(force=force)
    except FileNotFoundError as e:
        console.print(f"\n [bold red]✗ Error:[/bold red] {e}")
        sys.exit(1)
    except Exception as e:
        console.print(f"\n [bold red]✗ Unexpected error:[/bold red] {e}")
        raise

    elapsed = time.time() - start
    console.print()
    console.print(
        Panel(
            Text.assemble(
                Text(" ✦ ", style="bold bright_green"),
                Text("Indexing complete!\n", style="bold bright_green"),
                Text(f"   Time taken: {elapsed:.1f}s\n", style="dim white"),
                Text("   You can now run: ", style="dim white"),
                Text("uv run python app.py", style="bold bright_cyan"),
            ),
            box=box.ROUNDED,
            border_style="bright_green",
            padding=(0, 2),
        )
    )
    console.print()


if __name__ == "__main__":
    main()
