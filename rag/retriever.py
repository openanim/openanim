from __future__ import annotations
import chromadb
from pathlib import Path
from .embedder import embed_query
from .indexer import get_chroma_client, get_or_create_collection

TOP_K = 16
MAX_CHUNK_CHARS = 1200
SOURCE_PRIORITY = {"source_code": 0, "example": 1, "documentation": 2, "changelog": 3}


def retrieve_context(query, top_k=TOP_K, client=None, filter_source_types=None):
    if client is None:
        client = get_chroma_client()
    collection = get_or_create_collection(client)
    if collection.count() == 0:
        return "", []

    where = None
    if filter_source_types:
        where = (
            {"source_type": {"$eq": filter_source_types[0]}}
            if len(filter_source_types) == 1
            else {"source_type": {"$in": filter_source_types}}
        )

    results = collection.query(
        query_embeddings=[embed_query(query)],
        n_results=min(top_k, collection.count()),
        where=where,
        include=["documents", "metadatas", "distances"],
    )
    raw = [
        {
            "text": doc,
            "source_type": meta.get("source_type", "unknown"),
            "file_path": meta.get("file_path", ""),
            "version": meta.get("version", ""),
            "chunk_subtype": meta.get("chunk_subtype", ""),
            "similarity": round(1.0 - dist, 4),
        }
        for doc, meta, dist in zip(
            results["documents"][0], results["metadatas"][0], results["distances"][0]
        )
    ]
    raw.sort(key=lambda x: (SOURCE_PRIORITY.get(x["source_type"], 9), -x["similarity"]))

    parts = ["=== MANIM API CONTEXT ===\n"]
    for i, r in enumerate(raw):
        label = f"[{r['source_type'].upper()} | {Path(r['file_path']).name} | sim={r['similarity']:.3f}]"
        snippet = r["text"][:MAX_CHUNK_CHARS] + (
            "\n… (truncated)" if len(r["text"]) > MAX_CHUNK_CHARS else ""
        )
        parts.append(f"\n--- Chunk {i+1} {label} ---\n{snippet}")
    parts.append("\n=== END MANIM API CONTEXT ===")

    return "\n".join(parts), raw
