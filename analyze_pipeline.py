"""
analyze_pipeline.py
-------------------
Data Science analysis of the AutoManim pipeline logs.

Reads logs/pipeline_log.jsonl and generates:
  1. Statistical summary (mean, median, std of key metrics)
  2. Visualizations:
     - Success rate (with vs without RAG)
     - Error category distribution (bar chart)
     - Self-healing effectiveness (attempts to success)
     - RAG similarity score distribution (histogram)
     - Manim object category usage (horizontal bar)
     - Code complexity vs success (scatter)
  3. Correlation matrix of numeric features

Usage:
    uv run python analyze_pipeline.py

Outputs saved to: logs/analysis/
"""

import json
import sys
from pathlib import Path
from collections import Counter

try:
    import pandas as pd
    import matplotlib
    matplotlib.use("Agg")  # non-interactive backend
    import matplotlib.pyplot as plt
    import matplotlib.patches as mpatches
    import numpy as np
    from rich.console import Console
    from rich.table import Table
    from rich import box as rbox
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("Run: uv add pandas matplotlib numpy")
    sys.exit(1)

console = Console()
JSONL_PATH = Path("logs/pipeline_log.jsonl")
OUTPUT_DIR = Path("logs/analysis")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# ── Colour palette (matches AutoManim CLI aesthetic) ─────────────────────────
PALETTE = {
    "magenta": "#d946ef",
    "cyan": "#22d3ee",
    "green": "#4ade80",
    "yellow": "#fde68a",
    "red": "#f87171",
    "blue": "#60a5fa",
    "purple": "#a78bfa",
    "orange": "#fb923c",
}
BG = "#0f0f17"
PANEL = "#1a1a2e"
TEXT = "#e2e8f0"
GRID = "#2d2d44"

plt.rcParams.update({
    "figure.facecolor": BG,
    "axes.facecolor": PANEL,
    "axes.edgecolor": GRID,
    "axes.labelcolor": TEXT,
    "xtick.color": TEXT,
    "ytick.color": TEXT,
    "text.color": TEXT,
    "grid.color": GRID,
    "grid.linestyle": "--",
    "grid.alpha": 0.5,
    "font.family": "monospace",
    "axes.titlecolor": TEXT,
    "axes.titlesize": 13,
    "axes.labelsize": 11,
})


# ── Data Loading ──────────────────────────────────────────────────────────────

def load_sessions() -> list[dict]:
    if not JSONL_PATH.exists():
        console.print(f"[red]No log file found at {JSONL_PATH}[/red]")
        console.print("[dim]Run some animations with app.py first.[/dim]")
        sys.exit(0)
    sessions = []
    with JSONL_PATH.open() as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    sessions.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
    return sessions


def build_dataframe(sessions: list[dict]) -> pd.DataFrame:
    """Flatten session data into a tabular DataFrame."""
    rows = []
    for s in sessions:
        rag = s.get("rag_retrieval") or {}
        attempts = s.get("generation_attempts", [])

        # Per-attempt rows
        for att in attempts:
            code_m = att.get("code_metrics", {})
            rows.append({
                "session_id": s.get("session_id"),
                "prompt": s.get("prompt", "")[:60],
                "final_outcome": s.get("final_outcome"),
                "rag_used": s.get("rag_used", False),
                "rag_chunks": rag.get("chunks_retrieved", 0),
                "rag_mean_sim": rag.get("mean_similarity", 0.0),
                "total_duration_s": s.get("total_duration_s"),
                "total_attempts": s.get("total_attempts"),
                "heal_attempts": s.get("heal_attempts", 0),

                "attempt_number": att.get("attempt_number"),
                "is_heal_attempt": att.get("is_heal_attempt", False),
                "latency_s": att.get("latency_s", 0),
                "syntax_ok": att.get("validation", {}).get("syntax_ok"),
                "scene_class_ok": att.get("validation", {}).get("scene_class_ok"),
                "render_success": att.get("render_success"),
                "error_category": att.get("error_category"),

                "code_lines": code_m.get("total_lines", 0),
                "play_calls": code_m.get("self_play_calls", 0),
                "wait_calls": code_m.get("self_wait_calls", 0),
                "has_3d": code_m.get("has_3d", False),
                "has_tex": code_m.get("has_tex", False),
                "has_anim_group": code_m.get("has_animation_group", False),
                "lambda_count": code_m.get("lambda_count", 0),
                "obj_categories": ",".join(code_m.get("object_categories", [])),
                "ast_parse_ok": code_m.get("ast_parse_ok", True),
            })

    return pd.DataFrame(rows)


# ── Statistical Summary ───────────────────────────────────────────────────────

def print_stats(sessions: list[dict], df: pd.DataFrame):
    total = len(sessions)
    successes = sum(1 for s in sessions if s.get("final_outcome") == "success")
    rag_sessions = sum(1 for s in sessions if s.get("rag_used"))
    rag_successes = sum(
        1 for s in sessions if s.get("rag_used") and s.get("final_outcome") == "success"
    )
    no_rag_sessions = total - rag_sessions
    no_rag_successes = successes - rag_successes

    console.print()
    table = Table(
        title="[bold bright_magenta]AutoManim Pipeline Statistics[/bold bright_magenta]",
        box=rbox.ROUNDED,
        border_style="bright_magenta",
        show_header=True,
        header_style="bold bright_cyan",
    )
    table.add_column("Metric", style="bold white")
    table.add_column("Value", style="bright_green")

    table.add_row("Total sessions", str(total))
    table.add_row("Overall success rate", f"{successes/total*100:.1f}%" if total else "N/A")
    table.add_row(
        "RAG success rate",
        f"{rag_successes/rag_sessions*100:.1f}% ({rag_sessions} sessions)"
        if rag_sessions else "N/A",
    )
    table.add_row(
        "No-RAG success rate",
        f"{no_rag_successes/no_rag_sessions*100:.1f}% ({no_rag_sessions} sessions)"
        if no_rag_sessions else "N/A",
    )

    durations = [s.get("total_duration_s") for s in sessions if s.get("total_duration_s")]
    if durations:
        table.add_row("Mean duration", f"{np.mean(durations):.1f}s")
        table.add_row("Median duration", f"{np.median(durations):.1f}s")
        table.add_row("Std duration", f"{np.std(durations):.1f}s")

    attempts_col = df["total_attempts"].dropna()
    if len(attempts_col):
        table.add_row("Mean attempts/session", f"{attempts_col.mean():.2f}")
        table.add_row("Max attempts/session", str(int(attempts_col.max())))

    error_cats = df[df["error_category"].notna()]["error_category"].value_counts()
    if len(error_cats):
        top_err = error_cats.index[0]
        table.add_row("Most common error", f"{top_err} ({error_cats.iloc[0]}×)")

    console.print(table)
    console.print()


# ── Plots ─────────────────────────────────────────────────────────────────────

def _save(fig: plt.Figure, name: str):
    path = OUTPUT_DIR / name
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    console.print(f"  [dim]Saved[/dim] {path}")


def plot_success_rate_rag(sessions: list[dict]):
    """Bar chart: success rate with RAG vs without."""
    groups = {"With RAG": {"success": 0, "total": 0}, "Without RAG": {"success": 0, "total": 0}}
    for s in sessions:
        key = "With RAG" if s.get("rag_used") else "Without RAG"
        groups[key]["total"] += 1
        if s.get("final_outcome") == "success":
            groups[key]["success"] += 1

    labels = list(groups.keys())
    rates = [
        (g["success"] / g["total"] * 100) if g["total"] else 0
        for g in groups.values()
    ]
    counts = [g["total"] for g in groups.values()]
    colors = [PALETTE["cyan"], PALETTE["purple"]]

    fig, ax = plt.subplots(figsize=(7, 4))
    bars = ax.bar(labels, rates, color=colors, width=0.4, zorder=3)
    ax.set_ylim(0, 110)
    ax.set_ylabel("Success Rate (%)")
    ax.set_title("First-Pass Success Rate: RAG vs No-RAG")
    ax.yaxis.grid(True, zorder=0)
    for bar, rate, count in zip(bars, rates, counts):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + 2,
            f"{rate:.0f}%\n(n={count})",
            ha="center", va="bottom", color=TEXT, fontsize=10,
        )
    fig.tight_layout()
    _save(fig, "1_success_rate_rag.png")


def plot_error_distribution(df: pd.DataFrame):
    """Horizontal bar: error category distribution."""
    errors = df[df["error_category"].notna()]["error_category"].value_counts()
    if errors.empty:
        return

    fig, ax = plt.subplots(figsize=(8, 4))
    colors = list(PALETTE.values())[:len(errors)]
    bars = ax.barh(errors.index[::-1], errors.values[::-1], color=colors[::-1], zorder=3)
    ax.set_xlabel("Occurrences")
    ax.set_title("Error Category Distribution (Self-Healing Pipeline)")
    ax.xaxis.grid(True, zorder=0)
    for bar, val in zip(bars, errors.values[::-1]):
        ax.text(
            val + 0.1, bar.get_y() + bar.get_height() / 2,
            str(val), va="center", ha="left", color=TEXT, fontsize=9,
        )
    fig.tight_layout()
    _save(fig, "2_error_distribution.png")


def plot_healing_effectiveness(df: pd.DataFrame):
    """Bar: what attempt number led to success."""
    success_attempts = df[df["render_success"] == True]["attempt_number"].value_counts().sort_index()
    if success_attempts.empty:
        return

    fig, ax = plt.subplots(figsize=(7, 4))
    ax.bar(
        [f"Attempt {i}" for i in success_attempts.index],
        success_attempts.values,
        color=[PALETTE["green"], PALETTE["yellow"], PALETTE["orange"], PALETTE["red"]][:len(success_attempts)],
        zorder=3,
    )
    ax.set_ylabel("Times")
    ax.set_title("Self-Healing: Attempt Number that Achieved Success")
    ax.yaxis.grid(True, zorder=0)
    for i, (idx, val) in enumerate(success_attempts.items()):
        ax.text(i, val + 0.05, str(val), ha="center", va="bottom", color=TEXT, fontsize=10)
    fig.tight_layout()
    _save(fig, "3_healing_effectiveness.png")


def plot_rag_similarity_distribution(df: pd.DataFrame):
    """Histogram of RAG similarity scores."""
    sims = df[df["rag_mean_sim"] > 0]["rag_mean_sim"].dropna()
    if sims.empty:
        return

    fig, ax = plt.subplots(figsize=(7, 4))
    n, bins, patches = ax.hist(sims, bins=15, color=PALETTE["cyan"], edgecolor=BG, zorder=3, alpha=0.85)
    ax.axvline(sims.mean(), color=PALETTE["magenta"], linestyle="--", linewidth=1.5, label=f"Mean: {sims.mean():.3f}")
    ax.axvline(sims.median(), color=PALETTE["yellow"], linestyle=":", linewidth=1.5, label=f"Median: {sims.median():.3f}")
    ax.set_xlabel("Mean Cosine Similarity (RAG retrieval)")
    ax.set_ylabel("Frequency")
    ax.set_title("RAG Retrieval Quality — Similarity Score Distribution")
    ax.legend(facecolor=PANEL, edgecolor=GRID, labelcolor=TEXT)
    ax.yaxis.grid(True, zorder=0)
    fig.tight_layout()
    _save(fig, "4_rag_similarity_dist.png")


def plot_object_category_usage(df: pd.DataFrame):
    """Bar chart of Manim object category usage across all generated code."""
    all_cats = Counter()
    for cats_str in df["obj_categories"].dropna():
        for cat in cats_str.split(","):
            cat = cat.strip()
            if cat:
                all_cats[cat] += 1

    if not all_cats:
        return

    labels = list(all_cats.keys())
    values = list(all_cats.values())
    colors = list(PALETTE.values())[:len(labels)]

    fig, ax = plt.subplots(figsize=(9, 4))
    ax.bar(labels, values, color=colors, zorder=3)
    ax.set_ylabel("Sessions using category")
    ax.set_title("Manim Object Category Usage in Generated Code")
    ax.yaxis.grid(True, zorder=0)
    for i, v in enumerate(values):
        ax.text(i, v + 0.05, str(v), ha="center", va="bottom", color=TEXT, fontsize=9)
    fig.tight_layout()
    _save(fig, "5_object_category_usage.png")


def plot_code_complexity(df: pd.DataFrame):
    """Scatter: code lines vs play() calls, coloured by success."""
    subset = df[(df["attempt_number"] == 0) & df["code_lines"].notna() & df["play_calls"].notna()]
    if subset.empty:
        return

    success_mask = subset["render_success"] == True
    fail_mask = ~success_mask

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.scatter(
        subset[success_mask]["code_lines"],
        subset[success_mask]["play_calls"],
        c=PALETTE["green"], alpha=0.8, s=60, label="Success", zorder=3,
    )
    ax.scatter(
        subset[fail_mask]["code_lines"],
        subset[fail_mask]["play_calls"],
        c=PALETTE["red"], alpha=0.8, s=60, label="Failed", zorder=3, marker="x",
    )
    ax.set_xlabel("Generated Code Lines")
    ax.set_ylabel("self.play() Calls")
    ax.set_title("Code Complexity vs Render Success (attempt 0)")
    ax.legend(facecolor=PANEL, edgecolor=GRID, labelcolor=TEXT)
    ax.grid(True, zorder=0)
    fig.tight_layout()
    _save(fig, "6_complexity_vs_success.png")


def plot_correlation_matrix(df: pd.DataFrame):
    """Heatmap: correlation between numeric pipeline features."""
    numeric_cols = [
        "rag_chunks", "rag_mean_sim", "total_attempts",
        "code_lines", "play_calls", "wait_calls", "lambda_count",
        "latency_s",
    ]
    available = [c for c in numeric_cols if c in df.columns]
    sub = df[available].dropna()
    if sub.empty or len(sub) < 3:
        return

    corr = sub.corr()

    fig, ax = plt.subplots(figsize=(9, 7))
    im = ax.imshow(corr.values, cmap="RdYlGn", vmin=-1, vmax=1, aspect="auto")
    plt.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
    ax.set_xticks(range(len(available)))
    ax.set_yticks(range(len(available)))
    ax.set_xticklabels(available, rotation=35, ha="right", fontsize=8)
    ax.set_yticklabels(available, fontsize=8)
    ax.set_title("Feature Correlation Matrix")

    for i in range(len(available)):
        for j in range(len(available)):
            ax.text(j, i, f"{corr.iloc[i, j]:.2f}", ha="center", va="center",
                    color="black", fontsize=7, fontweight="bold")

    fig.tight_layout()
    _save(fig, "7_correlation_matrix.png")


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    console.print()
    console.print("[bold bright_magenta]AutoManim — Pipeline Data Analysis[/bold bright_magenta]")
    console.print(f"[dim]Reading: {JSONL_PATH}[/dim]\n")

    sessions = load_sessions()
    console.print(f"[bold]Loaded {len(sessions)} sessions[/bold]")

    df = build_dataframe(sessions)

    print_stats(sessions, df)

    console.print("[bold bright_cyan]Generating visualizations…[/bold bright_cyan]")
    plot_success_rate_rag(sessions)
    plot_error_distribution(df)
    plot_healing_effectiveness(df)
    plot_rag_similarity_distribution(df)
    plot_object_category_usage(df)
    plot_code_complexity(df)
    plot_correlation_matrix(df)

    console.print()
    console.print(
        f"[bold bright_green]✓ All plots saved to {OUTPUT_DIR}/[/bold bright_green]"
    )
    console.print()


if __name__ == "__main__":
    main()
