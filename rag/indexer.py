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
import subprocess
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
STATE_FILE = CHROMA_DB_PATH / ".rag_commit"

# Chunking parameters (optimized for extreme indexing depth and detail)
CHUNK_SIZE = 450         # characters per chunk (much smaller chunks isolate specific API traits better)
CHUNK_OVERLAP = 225      # massive overlap means fewer lost contextual boundaries between chunks
MIN_CHUNK_SIZE = 50      # keep small chunks even if they are only a line or two


def _get_current_commit() -> str | None:
    """Get the current git commit hash of the manim repository."""
    if not MANIM_REPO_PATH.exists():
        return None
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=MANIM_REPO_PATH,
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def _get_last_commit() -> str | None:
    """Read the last indexed commit hash from the state file."""
    if STATE_FILE.exists():
        return STATE_FILE.read_text(encoding="utf-8").strip()
    return None


def _get_save_commit(commit: str) -> None:
    """Save the commit hash to the state file."""
    CHROMA_DB_PATH.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(commit, encoding="utf-8")


def _get_changed_files(old_commit: str, new_commit: str) -> tuple[set[str], set[str]]:
    """Return a tuple of (modified_files, deleted_files) relative to the manim repo."""
    try:
        # Check files that were changed (added/modified/renamed)
        result_diff = subprocess.run(
            ["git", "diff", "--name-status", old_commit, new_commit],
            cwd=MANIM_REPO_PATH,
            capture_output=True,
            text=True,
            check=True
        )
        
        modified = set()
        deleted = set()
        
        for line in result_diff.stdout.strip().split("\n"):
            if not line:
                continue
            parts = line.split("\t")
            status, path = parts[0], parts[-1]
            
            if status.startswith("D"):
                deleted.add(path)
            else:
                modified.add(path)
                
        return modified, deleted
    except subprocess.CalledProcessError:
        # If the commit history is divergent or the old commit doesn't exist, fallback to rebuilding all
        return set(), set()


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
        # Cap method bodies at 150 lines to keep heavy, complex python methods entire.
        end = min(end, start + 150)
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


def _iter_corpus(allowed_paths: set[str] | None = None) -> Generator[dict, None, None]:
    """
    Walk the Manim repo and yield document dicts with keys:
        text, source_type, file_path, version
    If allowed_paths is provided, ONLY yield documents for those specific relative paths.
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
        # Ensure path uses forward slashes to match git diff output
        rel_path_git = rel_path.replace("\\", "/")
        
        if allowed_paths is not None and rel_path_git not in allowed_paths:
            continue

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
            rel_path_git = rel_path.replace("\\", "/")
            if allowed_paths is not None and rel_path_git not in allowed_paths:
                continue
                
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
        rel_path_git = rel_path.replace("\\", "/")
        if allowed_paths is not None and rel_path_git not in allowed_paths:
            continue
            
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
        rel_path_git = rel_path.replace("\\", "/")
        if allowed_paths is not None and rel_path_git not in allowed_paths:
            continue
            
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
    current_commit = _get_current_commit()
    last_commit = _get_last_commit()

    if is_indexed(client) and not force:
        if current_commit and last_commit and current_commit == last_commit:
            count = client.get_collection(COLLECTION_NAME).count()
            console.print(
                f"  [dim]RAG index is already up to date ({count:,} chunks at commit {current_commit[:7]}).[/dim]"
            )
            return

    modified_files = None
    deleted_files = None
    is_incremental = False
    
    if is_indexed(client) and not force and last_commit and current_commit:
        modified_files, deleted_files = _get_changed_files(last_commit, current_commit)
        # If we successfully parsed a diff, we can do an incremental update.
        if modified_files or deleted_files:
            is_incremental = True
            console.print(f"  [bold bright_cyan]Incremental update:[/bold bright_cyan] {len(modified_files)} modified, {len(deleted_files)} deleted files.")
        elif not modified_files and not deleted_files:
            # We got an empty diff but commits are different - probably an empty commit or metadata change.
            _get_save_commit(current_commit)
            console.print("  [dim]No relevant files changed in the update.[/dim]")
            return

    if force or (is_indexed(client) and not is_incremental):
        if not is_incremental and not force:
            console.print("  [dim]Full rebuild required (no prior commit state or divergent history).[/dim]")
        try:
            client.delete_collection(COLLECTION_NAME)
        except Exception:
            pass

    collection = get_or_create_collection(client)
    
    # --- Incremental Delete Phase ---
    if is_incremental and (modified_files or deleted_files):
        files_to_delete = modified_files.union(deleted_files)
        console.print(f"  [dim]Removing old chunks for {len(files_to_delete)} changed/deleted files…[/dim]")
        
        # We delete by file_path metadata. Note: Windows paths in metadata might have backslashes,
        # so we search for the base filename, or delete chunks where the metadata path matches our git diff path.
        # ChromaDB allows delete by metadata where field matches exactly.
        for file_path in files_to_delete:
            # Convert forward slashes to OS specific path separators for metadata matching
            os_specific_path = file_path.replace("/", os.sep)
            
            try:
                # Delete using the OS specific path we saved during indexing
                collection.delete(where={"file_path": os_specific_path})
            except Exception as e:
                pass


    # --- Indexing Phase ---
    if is_incremental:
        console.print(f"  [dim]Scanning {len(modified_files)} modified files…[/dim]")
        docs = list(_iter_corpus(allowed_paths=modified_files))
        if not docs:
            console.print("  [dim]No new chunks to embed after applying the diff filters.[/dim]")
            if current_commit:
                _get_save_commit(current_commit)
            return
    else:
        console.print("  [dim]Scanning entire Manim corpus…[/dim]")
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

    # Embed in batches with progress display (lowered since we have MANY more chunks)
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
        f"  [bold bright_green]✓[/bold bright_green] {'Updated' if is_incremental else 'Created'} index with {len(docs):,} new chunks "
        f"into ChromaDB at [dim]{CHROMA_DB_PATH}[/dim]"
    )

    if current_commit:
        _get_save_commit(current_commit)
        console.print(f"  [dim]Saved state at commit {current_commit[:7]}[/dim]")
