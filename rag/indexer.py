from __future__ import annotations

"""
indexer.py
----------
Ingests the Manim Community Edition source code, documentation, changelogs,
and example scenes into a ChromaDB vector store.

Corpus strategy (what we index and WHY):
  - manim/manim/**/*.py       → Actual API source: class names, method
                                 signatures, docstrings. Ground truth for
                                 "what exists in the installed version."
  - manim/docs/source/**/*.rst|*.md → Official documentation, tutorials,
                                 guides. Explains HOW to use the API.
  - manim/docs/source/changelog/**  → Version-specific breaking changes,
                                 deprecations, new features. Gives the LLM
                                 knowledge of API drift across versions.
  - manim/example_scenes/**/*.py    → Complete, runnable examples that show
                                 correct usage patterns.

Each document is split into overlapping chunks so that large files are
represented by multiple, semantically coherent pieces.

Metadata stored per chunk:
  - source_type: "source_code" | "documentation" | "changelog" | "example"
  - file_path:   relative path inside the manim repo
  - version:     extracted from changelog filename where applicable
  - chunk_index: position of this chunk within the file
"""

import os
import re
import ast
import hashlib
from pathlib import Path
from typing import Generator

import chromadb
from chromadb.config import Settings
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn

from .embedder import embed_texts

console = Console()

# ── Configuration ─────────────────────────────────────────────────────────────

CHROMA_DB_PATH = Path(__file__).parent.parent / "chroma_db"
COLLECTION_NAME = "manim_knowledge"
MANIM_REPO_PATH = Path(__file__).parent.parent / "manim"

# Chunking parameters
CHUNK_SIZE = 800          # characters per chunk
CHUNK_OVERLAP = 150       # overlap between consecutive chunks
MIN_CHUNK_SIZE = 100      # ignore chunks smaller than this


def _make_id(file_path: str, chunk_index: int) -> str:
    """Create a stable, unique ID for a chunk."""
    raw = f"{file_path}::{chunk_index}"
    return hashlib.md5(raw.encode()).hexdigest()


def _chunk_text(text: str, size: int = CHUNK_SIZE, overlap: int = CHUNK_OVERLAP) -> list[str]:
    """
    Split text into overlapping chunks of ~'size' characters.
    Tries to break on newlines to preserve semantic boundaries.
    """
    chunks = []
    start = 0
    while start < len(text):
        end = start + size
        if end < len(text):
            # Try to break on a newline within the last 20% of the chunk
            search_start = start + int(size * 0.8)
            newline_pos = text.rfind("\n", search_start, end)
            if newline_pos > search_start:
                end = newline_pos
        chunk = text[start:end].strip()
        if len(chunk) >= MIN_CHUNK_SIZE:
            chunks.append(chunk)
        start = end - overlap
    return chunks


def _extract_py_docstring_chunks(source: str, file_path: str) -> list[dict]:
    """
    For Python source files, extract structured chunks:
    - Module-level docstring
    - Class definition + docstring
    - Method/function definition + docstring + body (capped)

    Falls back to plain chunking on parse failure.
    """
    chunks = []
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []

    def _get_source_segment(node) -> str:
        lines = source.splitlines()
        start = node.lineno - 1
        end = getattr(node, "end_lineno", start + 30)
        # Cap method bodies at 60 lines so chunks stay manageable
        end = min(end, start + 60)
        return "\n".join(lines[start:end])

    # Module docstring
    module_doc = ast.get_docstring(tree)
    if module_doc:
        chunks.append({
            "text": f"# {file_path}\n\n{module_doc}",
            "type": "module_docstring",
        })

    for node in ast.walk(tree):
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            segment = _get_source_segment(node).strip()
            if len(segment) < MIN_CHUNK_SIZE:
                continue
            chunks.append({
                "text": segment,
                "type": "class_def" if isinstance(node, ast.ClassDef) else "function_def",
            })

    return chunks


def _iter_corpus() -> Generator[dict, None, None]:
    """
    Walk the Manim repo and yield document dicts with keys:
        text, source_type, file_path, version
    """
    if not MANIM_REPO_PATH.exists():
        raise FileNotFoundError(
            f"Manim repo not found at {MANIM_REPO_PATH}. "
            "Run: git clone --depth=1 https://github.com/ManimCommunity/manim.git"
        )

    # ── 1. Source code ────────────────────────────────────────────────────────
    source_root = MANIM_REPO_PATH / "manim"
    for py_file in source_root.rglob("*.py"):
        # Skip opengl backend (less commonly used) and __pycache__
        if "opengl" in py_file.parts or "__pycache__" in py_file.parts:
            continue
        try:
            source = py_file.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue

        rel_path = str(py_file.relative_to(MANIM_REPO_PATH))

        # Try AST-based extraction first
        ast_chunks = _extract_py_docstring_chunks(source, rel_path)
        if ast_chunks:
            for chunk in ast_chunks:
                yield {
                    "text": chunk["text"],
                    "source_type": "source_code",
                    "file_path": rel_path,
                    "version": "latest",
                    "chunk_subtype": chunk["type"],
                }
        else:
            # Fallback: plain chunking
            for chunk in _chunk_text(source):
                yield {
                    "text": chunk,
                    "source_type": "source_code",
                    "file_path": rel_path,
                    "version": "latest",
                    "chunk_subtype": "raw",
                }

    # ── 2. Documentation (RST / MD) ───────────────────────────────────────────
    docs_root = MANIM_REPO_PATH / "docs" / "source"
    doc_patterns = ["*.rst", "*.md"]
    for pattern in doc_patterns:
        for doc_file in docs_root.rglob(pattern):
            # Skip changelog dir — handled separately below
            if "changelog" in doc_file.parts:
                continue
            try:
                text = doc_file.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            rel_path = str(doc_file.relative_to(MANIM_REPO_PATH))
            for chunk in _chunk_text(text):
                yield {
                    "text": chunk,
                    "source_type": "documentation",
                    "file_path": rel_path,
                    "version": "latest",
                    "chunk_subtype": "docs",
                }

    # ── 3. Changelogs ─────────────────────────────────────────────────────────
    changelog_dir = MANIM_REPO_PATH / "docs" / "source" / "changelog"
    version_pattern = re.compile(r"(\d+\.\d+[\.\d]*)")
    for cl_file in sorted(changelog_dir.iterdir()):
        if cl_file.suffix not in (".rst", ".md"):
            continue
        version_match = version_pattern.search(cl_file.stem)
        version_str = version_match.group(1) if version_match else "unknown"
        try:
            text = cl_file.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        rel_path = str(cl_file.relative_to(MANIM_REPO_PATH))
        for chunk in _chunk_text(text):
            yield {
                "text": chunk,
                "source_type": "changelog",
                "file_path": rel_path,
                "version": version_str,
                "chunk_subtype": "changelog",
            }

    # ── 4. Example scenes ────────────────────────────────────────────────────
    examples_root = MANIM_REPO_PATH / "example_scenes"
    for ex_file in examples_root.rglob("*.py"):
        try:
            source = ex_file.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        rel_path = str(ex_file.relative_to(MANIM_REPO_PATH))
        for chunk in _chunk_text(source):
            yield {
                "text": chunk,
                "source_type": "example",
                "file_path": rel_path,
                "version": "latest",
                "chunk_subtype": "example",
            }


# ── Public API ────────────────────────────────────────────────────────────────


def get_chroma_client() -> chromadb.PersistentClient:
    return chromadb.PersistentClient(path=str(CHROMA_DB_PATH))


def get_or_create_collection(client: chromadb.PersistentClient) -> chromadb.Collection:
    return client.get_or_create_collection(
        name=COLLECTION_NAME,
        metadata={"hnsw:space": "cosine"},
    )


def is_indexed(client: chromadb.PersistentClient | None = None) -> bool:
    """Return True if the ChromaDB collection already has documents."""
    if client is None:
        client = get_chroma_client()
    try:
        col = client.get_collection(COLLECTION_NAME)
        return col.count() > 0
    except Exception:
        return False


def build_index(force: bool = False) -> None:
    """
    Ingest the Manim corpus into ChromaDB.

    Args:
        force: If True, drop and rebuild the collection even if it exists.
    """
    client = get_chroma_client()

    if is_indexed(client) and not force:
        count = client.get_collection(COLLECTION_NAME).count()
        console.print(
            f"  [dim]RAG index already exists ({count:,} chunks). "
            "Use force=True to rebuild.[/dim]"
        )
        return

    if force:
        try:
            client.delete_collection(COLLECTION_NAME)
        except Exception:
            pass

    collection = get_or_create_collection(client)

    # Collect all docs first so we know the total
    console.print("  [dim]Scanning Manim corpus…[/dim]")
    docs = list(_iter_corpus())
    console.print(f"  [dim]Found {len(docs):,} chunks to embed.[/dim]")

    texts = [d["text"] for d in docs]
    ids = [_make_id(d["file_path"], i) for i, d in enumerate(docs)]
    metadatas = [
        {
            "source_type": d["source_type"],
            "file_path": d["file_path"],
            "version": d["version"],
            "chunk_subtype": d.get("chunk_subtype", ""),
        }
        for d in docs
    ]

    # Embed in batches with progress display
    EMBED_BATCH = 64
    CHROMA_BATCH = 500

    all_embeddings: list[list[float]] = []
    with Progress(
        SpinnerColumn(),
        TextColumn("[bold bright_magenta]Embedding[/bold bright_magenta]"),
        BarColumn(),
        TaskProgressColumn(),
        TextColumn("[dim]{task.completed}/{task.total} texts[/dim]"),
        console=console,
    ) as progress:
        task = progress.add_task("", total=len(texts))
        for i in range(0, len(texts), EMBED_BATCH):
            batch = texts[i : i + EMBED_BATCH]
            batch_embeddings = embed_texts(batch)
            all_embeddings.extend(batch_embeddings)
            progress.update(task, advance=len(batch))

    # Insert into ChromaDB in batches
    console.print("  [dim]Writing to ChromaDB…[/dim]")
    with Progress(
        SpinnerColumn(),
        TextColumn("[bold bright_cyan]Indexing[/bold bright_cyan]"),
        BarColumn(),
        TaskProgressColumn(),
        console=console,
    ) as progress:
        task = progress.add_task("", total=len(docs))
        for i in range(0, len(docs), CHROMA_BATCH):
            collection.add(
                ids=ids[i : i + CHROMA_BATCH],
                embeddings=all_embeddings[i : i + CHROMA_BATCH],
                documents=texts[i : i + CHROMA_BATCH],
                metadatas=metadatas[i : i + CHROMA_BATCH],
            )
            progress.update(task, advance=min(CHROMA_BATCH, len(docs) - i))

    console.print(
        f"  [bold bright_green]✓[/bold bright_green] Indexed {len(docs):,} chunks "
        f"into ChromaDB at [dim]{CHROMA_DB_PATH}[/dim]"
    )
