from rag.retriever import retrieve_context
ctx, results = retrieve_context("animate a sine wave transforming into a cosine wave")
print(f"Retrieved {len(results)} chunks")
for r in results:
    print(f"  [{r['source_type']:12}] sim={r['similarity']:.3f}  {r['file_path'][-50:]}")
print()
print("Context preview (first 500 chars):")
print(ctx[:500])
