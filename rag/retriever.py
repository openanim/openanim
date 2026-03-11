"""
retriever.py
------------
Query the ChromaDB vector store and return formatted context strings
that are injected into the LLM system prompt.

Retrieval strategy:
  - Embed the user's natural language prompt
  - Search the collection for the top-K most similar chunks
  - Filter and rank results by source_type priority:
      source_code > example > documentation > changelog
  - Format into a concise context block the LLM can use
"""

from __future__ import annotations

import chromadb
from rich.console import Console

from .embedder import embed_query
from .indexer import get_chroma_client, get_or_create_collection, COLLECTION_NAME

console = Console()

# Number of chunks to retrieve
TOP_K = 8

# How many characters of each chunk to include in context (cap long chunks)
MAX_CHUNK_CHARS = 600

# Source type priority (lower = higher priority in display order)
SOURCE_PRIORITY = {
    "source_code": 0,
    "example": 1,
    "documentation": 2,
    "changelog": 3,
}


def retrieve_context(
    query: str,
    top_k: int = TOP_K,
    client: chromadb.PersistentClient | None = None,
    filter_source_types: list[str] | None = None,
) -> tuple[str, list[dict]]:
    """
    Retrieve the most relevant Manim knowledge chunks for a given query.

    Args:
        query:              Natural language prompt from the user.
        top_k:              Number of chunks to retrieve.
        client:             Optional ChromaDB client (created if None).
        filter_source_types: Optional list e.g. ["source_code", "example"]
                             to restrict search to specific corpus sections.

    Returns:
        A tuple of:
          - formatted_context:  A string ready to inject into the LLM prompt.
          - raw_results:        List of result dicts (for logging / analysis).
    """
    if client is None:
        client = get_chroma_client()

    collection = get_or_create_collection(client)

    if collection.count() == 0:
        # Index not built yet — return empty context
        return "", []

    # Embed the query
    query_embedding = embed_query(query)

    # Build optional where filter
    where = None
    if filter_source_types:
        if len(filter_source_types) == 1:
            where = {"source_type": {"$eq": filter_source_types[0]}}
        else:
            where = {"source_type": {"$in": filter_source_types}}

    # Query ChromaDB
    results = collection.query(
        query_embeddings=[query_embedding],
        n_results=min(top_k, collection.count()),
        where=where,
        include=["documents", "metadatas", "distances"],
    )

    # Unpack results
    raw: list[dict] = []
    docs = results["documents"][0]
    metas = results["metadatas"][0]
    distances = results["distances"][0]

    for doc, meta, dist in zip(docs, metas, distances):
        raw.append({
            "text": doc,
            "source_type": meta.get("source_type", "unknown"),
            "file_path": meta.get("file_path", ""),
            "version": meta.get("version", ""),
            "chunk_subtype": meta.get("chunk_subtype", ""),
            "similarity": round(1.0 - dist, 4),  # cosine distance → similarity
        })

    # Sort by source priority, then similarity
    raw.sort(key=lambda x: (SOURCE_PRIORITY.get(x["source_type"], 9), -x["similarity"]))

    # Format context block for LLM injection
    context_parts = ["=== MANIM API CONTEXT (retrieved from source) ===\n"]
    for i, r in enumerate(raw):
        label = f"[{r['source_type'].upper()} | {Path_stem(r['file_path'])} | sim={r['similarity']:.3f}]"
        snippet = r["text"][:MAX_CHUNK_CHARS]
        if len(r["text"]) > MAX_CHUNK_CHARS:
            snippet += "\n… (truncated)"
        context_parts.append(f"\n--- Chunk {i+1} {label} ---\n{snippet}")

    context_parts.append("\n=== END MANIM API CONTEXT ===")
    formatted = "\n".join(context_parts)

    return formatted, raw


def Path_stem(path_str: str) -> str:
    """Get just the filename stem from a path string."""
    from pathlib import Path
    try:
        return Path(path_str).name
    except Exception:
        return path_str
