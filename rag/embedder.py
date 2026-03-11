"""
embedder.py
-----------
OpenRouter Embeddings client for AutoManim RAG.

Uses nomic-ai/nomic-embed-text via OpenRouter's OpenAI-compatible
embeddings endpoint.  Batches inputs to stay under the API's per-request
limit and returns plain Python lists so ChromaDB can store them directly.
"""

import os
import time
from typing import Union
import requests
from dotenv import load_dotenv

load_dotenv()

# Default model — OpenAI text-embedding-3-small via OpenRouter, 1536-dim
# Confirmed working: text-embedding-3-small (also: openai/text-embedding-3-small)
EMBED_MODEL = "text-embedding-3-small"

# Max texts per API call (be conservative)
BATCH_SIZE = 64

OPENROUTER_API_BASE = "https://openrouter.ai/api/v1"


def _embed_batch(texts: list[str], api_key: str, model: str) -> list[list[float]]:
    """
    Call OpenRouter /embeddings for a single batch of texts.
    Returns a list of embedding vectors (list-of-floats).
    """
    response = requests.post(
        f"{OPENROUTER_API_BASE}/embeddings",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/AutoManim",
            "X-Title": "AutoManim",
        },
        json={
            "model": model,
            "input": texts,
            "encoding_format": "float",
        },
        timeout=120,
    )
    response.raise_for_status()
    data = response.json()
    # OpenAI-compatible: data["data"] sorted by index
    sorted_items = sorted(data["data"], key=lambda x: x["index"])
    return [item["embedding"] for item in sorted_items]


def embed_texts(
    texts: list[str],
    model: str = EMBED_MODEL,
    api_key: str | None = None,
    retry_delay: float = 2.0,
    max_retries: int = 3,
) -> list[list[float]]:
    """
    Embed a list of texts using OpenRouter.

    Handles batching automatically.  Returns a list of embedding vectors
    in the same order as the input texts.
    """
    if not api_key:
        api_key = os.getenv("OPENROUTER_API_KEY")
    if not api_key:
        raise ValueError("OPENROUTER_API_KEY is not set.")

    all_embeddings: list[list[float]] = []

    for i in range(0, len(texts), BATCH_SIZE):
        batch = texts[i : i + BATCH_SIZE]
        for attempt in range(max_retries):
            try:
                batch_embeddings = _embed_batch(batch, api_key, model)
                all_embeddings.extend(batch_embeddings)
                break
            except requests.HTTPError as e:
                if attempt < max_retries - 1:
                    time.sleep(retry_delay * (attempt + 1))
                else:
                    raise RuntimeError(
                        f"Embedding batch {i // BATCH_SIZE} failed after "
                        f"{max_retries} attempts: {e}"
                    ) from e

    return all_embeddings


def embed_query(query: str, model: str = EMBED_MODEL, api_key: str | None = None) -> list[float]:
    """Convenience wrapper: embed a single query string."""
    return embed_texts([query], model=model, api_key=api_key)[0]
