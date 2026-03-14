from __future__ import annotations
import os, re, ast, hashlib, subprocess
from pathlib import Path
from typing import Generator
import chromadb
from rich.console import Console
from rich.progress import (
    Progress,
    SpinnerColumn,
    TextColumn,
    BarColumn,
    TaskProgressColumn,
)
from .embedder import embed_texts

console = Console()
CHROMA_DB_PATH = Path(__file__).parent.parent / "chroma_db"
COLLECTION_NAME = "manim_knowledge"
MANIM_REPO_PATH = Path(__file__).parent.parent / "manim"
STATE_FILE = CHROMA_DB_PATH / ".rag_commit"
CHUNK_SIZE, CHUNK_OVERLAP, MIN_CHUNK_SIZE = 450, 225, 50


def get_chroma_client():
    return chromadb.PersistentClient(path=str(CHROMA_DB_PATH))


def get_or_create_collection(client):
    return client.get_or_create_collection(
        COLLECTION_NAME, metadata={"hnsw:space": "cosine"}
    )


def is_indexed(client=None):
    try:
        return (client or get_chroma_client()).get_collection(
            COLLECTION_NAME
        ).count() > 0
    except Exception:
        return False


def _git(*args, cwd=MANIM_REPO_PATH):
    try:
        return subprocess.run(
            ["git"] + list(args), cwd=cwd, capture_output=True, text=True, check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def _get_current_commit():
    return _git("rev-parse", "HEAD") if MANIM_REPO_PATH.exists() else None


def _get_last_commit():
    return (
        STATE_FILE.read_text(encoding="utf-8").strip() if STATE_FILE.exists() else None
    )


def _save_commit(commit):
    CHROMA_DB_PATH.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(commit, encoding="utf-8")


def _get_changed_files(old, new):
    out = _git("diff", "--name-status", old, new)
    if not out:
        return set(), set()
    modified, deleted = set(), set()
    for line in out.split("\n"):
        if not line:
            continue
        parts = line.split("\t")
        (deleted if parts[0].startswith("D") else modified).add(parts[-1])
    return modified, deleted


def _make_id(file_path, chunk_index):
    return hashlib.md5(f"{file_path}::{chunk_index}".encode()).hexdigest()


def _chunk_text(text, size=CHUNK_SIZE, overlap=CHUNK_OVERLAP):
    chunks, start = [], 0
    while start < len(text):
        end = start + size
        if end < len(text):
            nl = text.rfind("\n", start + int(size * 0.8), end)
            if nl > start + int(size * 0.8):
                end = nl
        chunk = text[start:end].strip()
        if len(chunk) >= MIN_CHUNK_SIZE:
            chunks.append(chunk)
        start = end - overlap
    return chunks


def _extract_py_docstring_chunks(source, file_path):
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []
    lines = source.splitlines()
    chunks = []
    doc = ast.get_docstring(tree)
    if doc:
        chunks.append({"text": f"# {file_path}\n\n{doc}", "type": "module_docstring"})
    for node in ast.walk(tree):
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            end = min(
                getattr(node, "end_lineno", node.lineno + 30), node.lineno - 1 + 150
            )
            segment = "\n".join(lines[node.lineno - 1 : end]).strip()
            if len(segment) >= MIN_CHUNK_SIZE:
                chunks.append(
                    {
                        "text": segment,
                        "type": (
                            "class_def"
                            if isinstance(node, ast.ClassDef)
                            else "function_def"
                        ),
                    }
                )
    return chunks


def _iter_corpus(allowed_paths=None):
    if not MANIM_REPO_PATH.exists():
        raise FileNotFoundError(
            f"Manim repo not found at {MANIM_REPO_PATH}. Run: git clone --depth=1 https://github.com/ManimCommunity/manim.git"
        )

    def _rel(p, root=MANIM_REPO_PATH):
        r = str(p.relative_to(root))
        return r, r.replace("\\", "/")

    def _allowed(git_path):
        return allowed_paths is None or git_path in allowed_paths

    def _read(p):
        try:
            return p.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            return None

    # 1. Source code
    for py in (MANIM_REPO_PATH / "manim").rglob("*.py"):
        if "opengl" in py.parts or "__pycache__" in py.parts:
            continue
        src = _read(py)
        if not src:
            continue
        rel, git = _rel(py)
        if not _allowed(git):
            continue
        ast_chunks = _extract_py_docstring_chunks(src, rel)
        for chunk in ast_chunks or [
            {"text": c, "type": "raw"} for c in _chunk_text(src)
        ]:
            yield {
                "text": chunk["text"],
                "source_type": "source_code",
                "file_path": rel,
                "version": "latest",
                "chunk_subtype": chunk["type"],
            }

    # 2. Docs
    for ext in ("*.rst", "*.md"):
        for doc in (MANIM_REPO_PATH / "docs" / "source").rglob(ext):
            if "changelog" in doc.parts:
                continue
            text = _read(doc)
            if not text:
                continue
            rel, git = _rel(doc)
            if not _allowed(git):
                continue
            for chunk in _chunk_text(text):
                yield {
                    "text": chunk,
                    "source_type": "documentation",
                    "file_path": rel,
                    "version": "latest",
                    "chunk_subtype": "docs",
                }

    # 3. Changelogs
    ver_re = re.compile(r"(\d+\.\d+[\.\d]*)")
    for cl in sorted((MANIM_REPO_PATH / "docs" / "source" / "changelog").iterdir()):
        if cl.suffix not in (".rst", ".md"):
            continue
        m = ver_re.search(cl.stem)
        version = m.group(1) if m else "unknown"
        text = _read(cl)
        if not text:
            continue
        rel, git = _rel(cl)
        if not _allowed(git):
            continue
        for chunk in _chunk_text(text):
            yield {
                "text": chunk,
                "source_type": "changelog",
                "file_path": rel,
                "version": version,
                "chunk_subtype": "changelog",
            }

    # 4. Examples
    for ex in (MANIM_REPO_PATH / "example_scenes").rglob("*.py"):
        src = _read(ex)
        if not src:
            continue
        rel, git = _rel(ex)
        if not _allowed(git):
            continue
        for chunk in _chunk_text(src):
            yield {
                "text": chunk,
                "source_type": "example",
                "file_path": rel,
                "version": "latest",
                "chunk_subtype": "example",
            }


def build_index(force=False):
    client = get_chroma_client()
    current, last = _get_current_commit(), _get_last_commit()

    if is_indexed(client) and not force:
        if current and current == last:
            count = client.get_collection(COLLECTION_NAME).count()
            console.print(
                f"  [dim]Index up to date ({count:,} chunks at {current[:7]}).[/dim]"
            )
            return

    modified = deleted = None
    is_incremental = False
    if is_indexed(client) and not force and last and current:
        modified, deleted = _get_changed_files(last, current)
        if modified or deleted:
            is_incremental = True
            console.print(
                f"  [bold bright_cyan]Incremental:[/bold bright_cyan] {len(modified)} modified, {len(deleted)} deleted."
            )
        else:
            _save_commit(current)
            console.print("  [dim]No relevant changes.[/dim]")
            return

    if not is_incremental:
        try:
            client.delete_collection(COLLECTION_NAME)
        except Exception:
            pass

    collection = get_or_create_collection(client)

    if is_incremental:
        for fp in (modified or set()) | (deleted or set()):
            try:
                collection.delete(where={"file_path": fp.replace("/", os.sep)})
            except Exception:
                pass

    allowed = modified if is_incremental else None
    docs = list(_iter_corpus(allowed_paths=allowed))
    console.print(f"  [dim]{len(docs):,} chunks to embed.[/dim]")

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

    EMBED_BATCH, CHROMA_BATCH = 64, 500
    all_embeddings = []
    with Progress(
        SpinnerColumn(),
        TextColumn("[bold bright_magenta]Embedding[/bold bright_magenta]"),
        BarColumn(),
        TaskProgressColumn(),
        console=console,
    ) as progress:
        task = progress.add_task("", total=len(texts))
        for i in range(0, len(texts), EMBED_BATCH):
            all_embeddings.extend(embed_texts(texts[i : i + EMBED_BATCH]))
            progress.update(task, advance=min(EMBED_BATCH, len(texts) - i))

    with Progress(
        SpinnerColumn(),
        TextColumn("[bold bright_cyan]Indexing[/bold bright_cyan]"),
        BarColumn(),
        TaskProgressColumn(),
        console=console,
    ) as progress:
        task = progress.add_task("", total=len(docs))
        for i in range(0, len(docs), CHROMA_BATCH):
            sl = slice(i, i + CHROMA_BATCH)
            collection.add(
                ids=ids[sl],
                embeddings=all_embeddings[sl],
                documents=texts[sl],
                metadatas=metadatas[sl],
            )
            progress.update(task, advance=min(CHROMA_BATCH, len(docs) - i))

    console.print(
        f"  [bold bright_green]✓[/bold bright_green] {'Updated' if is_incremental else 'Created'} index with {len(docs):,} chunks."
    )
    if current:
        _save_commit(current)
