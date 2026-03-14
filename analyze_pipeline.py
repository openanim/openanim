import json, sys
from pathlib import Path
from collections import Counter

try:
    import pandas as pd
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
    from rich.console import Console
    from rich.table import Table
    from rich import box as rbox
except ImportError as e:
    print(f"Missing dependency: {e}\nRun: uv add pandas matplotlib numpy")
    sys.exit(1)

console = Console()
JSONL_PATH = Path("logs/pipeline_log.jsonl")
OUTPUT_DIR = Path("logs/analysis")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

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
BG, PANEL, TEXT, GRID = "#0f0f17", "#1a1a2e", "#e2e8f0", "#2d2d44"

plt.rcParams.update(
    {
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
        "axes.titlesize": 13,
        "axes.labelsize": 11,
    }
)


def load_sessions():
    if not JSONL_PATH.exists():
        console.print(f"[red]No log at {JSONL_PATH}. Run some animations first.[/red]")
        sys.exit(0)
    sessions = []
    with JSONL_PATH.open() as f:
        for line in f:
            try:
                if line.strip():
                    sessions.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return sessions


def build_dataframe(sessions):
    rows = []
    for s in sessions:
        rag = s.get("rag_retrieval") or {}
        for att in s.get("generation_attempts", []):
            cm = att.get("code_metrics", {})
            rows.append(
                {
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
                    "code_lines": cm.get("total_lines", 0),
                    "play_calls": cm.get("self_play_calls", 0),
                    "wait_calls": cm.get("self_wait_calls", 0),
                    "has_3d": cm.get("has_3d", False),
                    "has_tex": cm.get("has_tex", False),
                    "lambda_count": cm.get("lambda_count", 0),
                    "obj_categories": ",".join(cm.get("object_categories", [])),
                    "ast_parse_ok": cm.get("ast_parse_ok", True),
                }
            )
    return pd.DataFrame(rows)


def print_stats(sessions, df):
    total = len(sessions)
    successes = sum(1 for s in sessions if s.get("final_outcome") == "success")
    rag_s = [s for s in sessions if s.get("rag_used")]
    no_rag_s = [s for s in sessions if not s.get("rag_used")]
    rag_ok = sum(1 for s in rag_s if s.get("final_outcome") == "success")
    no_rag_ok = sum(1 for s in no_rag_s if s.get("final_outcome") == "success")

    table = Table(
        title="[bold bright_magenta]Pipeline Statistics[/bold bright_magenta]",
        box=rbox.ROUNDED,
        border_style="bright_magenta",
        header_style="bold bright_cyan",
    )
    table.add_column("Metric", style="bold white")
    table.add_column("Value", style="bright_green")
    table.add_row("Total sessions", str(total))
    table.add_row(
        "Overall success rate", f"{successes/total*100:.1f}%" if total else "N/A"
    )
    table.add_row(
        "RAG success rate",
        f"{rag_ok/len(rag_s)*100:.1f}% (n={len(rag_s)})" if rag_s else "N/A",
    )
    table.add_row(
        "No-RAG success rate",
        (
            f"{no_rag_ok/len(no_rag_s)*100:.1f}% (n={len(no_rag_s)})"
            if no_rag_s
            else "N/A"
        ),
    )
    durs = [s.get("total_duration_s") for s in sessions if s.get("total_duration_s")]
    if durs:
        table.add_row(
            "Mean / median / std duration",
            f"{np.mean(durs):.1f}s / {np.median(durs):.1f}s / {np.std(durs):.1f}s",
        )
    atts = df["total_attempts"].dropna()
    if len(atts):
        table.add_row("Mean / max attempts", f"{atts.mean():.2f} / {int(atts.max())}")
    err_cats = df[df["error_category"].notna()]["error_category"].value_counts()
    if len(err_cats):
        table.add_row("Most common error", f"{err_cats.index[0]} ({err_cats.iloc[0]}×)")
    console.print(table)


def _save(fig, name):
    path = OUTPUT_DIR / name
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    console.print(f"  [dim]Saved[/dim] {path}")


def _bar_labels(ax, bars, values, fmt=str):
    for bar, val in zip(bars, values):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + 0.05 * max(values),
            fmt(val),
            ha="center",
            va="bottom",
            color=TEXT,
            fontsize=9,
        )


def plot_success_rate_rag(sessions):
    groups = {"With RAG": {"s": 0, "n": 0}, "Without RAG": {"s": 0, "n": 0}}
    for s in sessions:
        g = groups["With RAG" if s.get("rag_used") else "Without RAG"]
        g["n"] += 1
        if s.get("final_outcome") == "success":
            g["s"] += 1
    labels = list(groups.keys())
    rates = [(g["s"] / g["n"] * 100) if g["n"] else 0 for g in groups.values()]
    fig, ax = plt.subplots(figsize=(7, 4))
    bars = ax.bar(
        labels, rates, color=[PALETTE["cyan"], PALETTE["purple"]], width=0.4, zorder=3
    )
    ax.set_ylim(0, 110)
    ax.set_ylabel("Success Rate (%)")
    ax.set_title("Success Rate: RAG vs No-RAG")
    for bar, rate, g in zip(bars, rates, groups.values()):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + 2,
            f"{rate:.0f}%\n(n={g['n']})",
            ha="center",
            va="bottom",
            color=TEXT,
            fontsize=10,
        )
    fig.tight_layout()
    _save(fig, "1_success_rate_rag.png")


def plot_error_distribution(df):
    errors = df[df["error_category"].notna()]["error_category"].value_counts()
    if errors.empty:
        return
    colors = list(PALETTE.values())[: len(errors)]
    fig, ax = plt.subplots(figsize=(8, 4))
    bars = ax.barh(
        errors.index[::-1], errors.values[::-1], color=colors[::-1], zorder=3
    )
    ax.set_xlabel("Occurrences")
    ax.set_title("Error Category Distribution")
    for bar, val in zip(bars, errors.values[::-1]):
        ax.text(
            val + 0.1,
            bar.get_y() + bar.get_height() / 2,
            str(val),
            va="center",
            color=TEXT,
            fontsize=9,
        )
    fig.tight_layout()
    _save(fig, "2_error_distribution.png")


def plot_healing_effectiveness(df):
    sa = df[df["render_success"] == True]["attempt_number"].value_counts().sort_index()
    if sa.empty:
        return
    colors = [PALETTE["green"], PALETTE["yellow"], PALETTE["orange"], PALETTE["red"]][
        : len(sa)
    ]
    fig, ax = plt.subplots(figsize=(7, 4))
    bars = ax.bar([f"Attempt {i}" for i in sa.index], sa.values, color=colors, zorder=3)
    ax.set_ylabel("Times")
    ax.set_title("Attempt Number that Achieved Success")
    _bar_labels(ax, bars, sa.values)
    fig.tight_layout()
    _save(fig, "3_healing_effectiveness.png")


def plot_rag_similarity_distribution(df):
    sims = df[df["rag_mean_sim"] > 0]["rag_mean_sim"].dropna()
    if sims.empty:
        return
    fig, ax = plt.subplots(figsize=(7, 4))
    ax.hist(sims, bins=15, color=PALETTE["cyan"], edgecolor=BG, zorder=3, alpha=0.85)
    ax.axvline(
        sims.mean(),
        color=PALETTE["magenta"],
        linestyle="--",
        linewidth=1.5,
        label=f"Mean: {sims.mean():.3f}",
    )
    ax.axvline(
        sims.median(),
        color=PALETTE["yellow"],
        linestyle=":",
        linewidth=1.5,
        label=f"Median: {sims.median():.3f}",
    )
    ax.set_xlabel("Mean Cosine Similarity")
    ax.set_ylabel("Frequency")
    ax.set_title("RAG Similarity Distribution")
    ax.legend(facecolor=PANEL, edgecolor=GRID)
    fig.tight_layout()
    _save(fig, "4_rag_similarity_dist.png")


def plot_object_category_usage(df):
    all_cats = Counter(
        cat.strip()
        for cats in df["obj_categories"].dropna()
        for cat in cats.split(",")
        if cat.strip()
    )
    if not all_cats:
        return
    labels, values = zip(*all_cats.items())
    fig, ax = plt.subplots(figsize=(9, 4))
    bars = ax.bar(labels, values, color=list(PALETTE.values())[: len(labels)], zorder=3)
    ax.set_ylabel("Sessions using category")
    ax.set_title("Manim Object Category Usage")
    _bar_labels(ax, bars, values)
    fig.tight_layout()
    _save(fig, "5_object_category_usage.png")


def plot_code_complexity(df):
    sub = df[
        (df["attempt_number"] == 0)
        & df["code_lines"].notna()
        & df["play_calls"].notna()
    ]
    if sub.empty:
        return
    sm, fm = sub["render_success"] == True, sub["render_success"] != True
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.scatter(
        sub[sm]["code_lines"],
        sub[sm]["play_calls"],
        c=PALETTE["green"],
        alpha=0.8,
        s=60,
        label="Success",
        zorder=3,
    )
    ax.scatter(
        sub[fm]["code_lines"],
        sub[fm]["play_calls"],
        c=PALETTE["red"],
        alpha=0.8,
        s=60,
        label="Failed",
        zorder=3,
        marker="x",
    )
    ax.set_xlabel("Code Lines")
    ax.set_ylabel("self.play() Calls")
    ax.set_title("Code Complexity vs Render Success")
    ax.legend(facecolor=PANEL, edgecolor=GRID)
    fig.tight_layout()
    _save(fig, "6_complexity_vs_success.png")


def plot_correlation_matrix(df):
    cols = [
        "rag_chunks",
        "rag_mean_sim",
        "total_attempts",
        "code_lines",
        "play_calls",
        "wait_calls",
        "lambda_count",
        "latency_s",
    ]
    sub = df[[c for c in cols if c in df.columns]].dropna()
    if len(sub) < 3:
        return
    corr = sub.corr()
    fig, ax = plt.subplots(figsize=(9, 7))
    im = ax.imshow(corr.values, cmap="RdYlGn", vmin=-1, vmax=1)
    plt.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
    n = len(corr)
    ax.set_xticks(range(n))
    ax.set_yticks(range(n))
    ax.set_xticklabels(corr.columns, rotation=35, ha="right", fontsize=8)
    ax.set_yticklabels(corr.columns, fontsize=8)
    ax.set_title("Feature Correlation Matrix")
    for i in range(n):
        for j in range(n):
            ax.text(
                j,
                i,
                f"{corr.iloc[i,j]:.2f}",
                ha="center",
                va="center",
                color="black",
                fontsize=7,
                fontweight="bold",
            )
    fig.tight_layout()
    _save(fig, "7_correlation_matrix.png")


def main():
    console.print(
        "\n[bold bright_magenta]AutoManim — Pipeline Analysis[/bold bright_magenta]"
    )
    sessions = load_sessions()
    console.print(f"[bold]Loaded {len(sessions)} sessions[/bold]")
    df = build_dataframe(sessions)
    print_stats(sessions, df)
    console.print("[bold bright_cyan]Generating visualizations…[/bold bright_cyan]")
    for fn in [
        plot_success_rate_rag,
        plot_error_distribution,
        plot_healing_effectiveness,
        plot_rag_similarity_distribution,
        plot_object_category_usage,
        plot_code_complexity,
        plot_correlation_matrix,
    ]:
        fn(sessions if fn.__code__.co_varnames[0] == "sessions" else df)
    console.print(
        f"\n[bold bright_green]✓ All plots saved to {OUTPUT_DIR}/[/bold bright_green]"
    )


if __name__ == "__main__":
    main()
