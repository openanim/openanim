import os, time, requests, concurrent.futures
from dotenv import load_dotenv

load_dotenv()

EMBED_MODEL = "text-embedding-3-small"
BATCH_SIZE = 250
OPENROUTER_API_BASE = "https://openrouter.ai/api/v1"


def _embed_batch(texts, api_key, model=EMBED_MODEL):
    resp = requests.post(
        f"{OPENROUTER_API_BASE}/embeddings",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/OpenAnim",
            "X-Title": "OpenAnim",
        },
        json={"model": model, "input": texts, "encoding_format": "float"},
        timeout=120,
    )
    resp.raise_for_status()
    return [
        x["embedding"] for x in sorted(resp.json()["data"], key=lambda x: x["index"])
    ]


def embed_texts(
    texts,
    model=EMBED_MODEL,
    api_key=None,
    retry_delay=2.0,
    max_retries=3,
    max_workers=5,
):
    api_key = api_key or os.getenv("OPENROUTER_API_KEY")
    if not api_key:
        raise ValueError("OPENROUTER_API_KEY is not set.")
    all_embeddings = [[] for _ in range(len(texts))]
    batches = [(i, texts[i : i + BATCH_SIZE]) for i in range(0, len(texts), BATCH_SIZE)]

    def _process(i, batch):
        for attempt in range(max_retries):
            try:
                return i, _embed_batch(batch, api_key, model)
            except requests.HTTPError as e:
                if attempt < max_retries - 1:
                    time.sleep(retry_delay * (attempt + 1))
                else:
                    raise RuntimeError(
                        f"Batch {i//BATCH_SIZE} failed after {max_retries} attempts: {e}"
                    ) from e

    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as ex:
        for future in concurrent.futures.as_completed(
            {ex.submit(_process, i, b): (i, b) for i, b in batches}
        ):
            i, embs = future.result()
            all_embeddings[i : i + len(embs)] = embs
    return all_embeddings


def embed_query(query, model=EMBED_MODEL, api_key=None):
    return embed_texts([query], model=model, api_key=api_key)[0]
