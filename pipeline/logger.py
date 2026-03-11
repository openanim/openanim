"""
logger.py
---------
Pipeline event logger for AutoManim.

Records structured JSON logs of every code generation session for
data science analysis:
  - Query text + timestamp
  - Retrieved RAG chunks (source type, similarity scores)
  - Generated code metrics (lines, classes detected, manim object types)
  - Validation outcomes (syntax ok, scene class ok)
  - Render outcomes (success/failure, attempt number, Manim error type)
  - Self-healing events (attempt number, error category, fix outcome)
  - Token/latency info where available

Logs are appended to:   logs/pipeline_log.jsonl
Session summary saved:  logs/sessions/session_{timestamp}.json

The JSONL format lets you load the entire history with pandas for analysis.
"""

import json
import re
import ast
import time
from datetime import datetime
from pathlib import Path
from typing import Any


LOG_DIR = Path(__file__).parent.parent / "logs"
JSONL_PATH = LOG_DIR / "pipeline_log.jsonl"
SESSION_DIR = LOG_DIR / "sessions"

# ── Manim object taxonomy (for data science categorical analysis) ──────────────
# Based on Manim CE v0.19+ class hierarchy
MANIM_CATEGORIES = {
    "geometry": [
        "Circle", "Square", "Rectangle", "Triangle", "Polygon", "RegularPolygon",
        "Line", "Arrow", "DoubleArrow", "DashedLine", "Dot", "Cross", "Ellipse",
        "Arc", "AnnularSector", "Sector", "Annulus", "CurvedArrow",
    ],
    "text": [
        "Text", "Tex", "MathTex", "MarkupText", "BraceLabel", "Brace",
        "BulletedList", "Title", "Paragraph",
    ],
    "graphing": [
        "Axes", "NumberPlane", "PolarPlane", "ComplexPlane", "NumberLine",
        "BarChart", "FunctionGraph", "ParametricCurve", "ImplicitFunction",
        "ValueTracker",
    ],
    "three_d": [
        "ThreeDScene", "Surface", "Sphere", "Cube", "Cone", "Cylinder",
        "Torus", "Dodecahedron", "Arrow3D", "Line3D",
    ],
    "animation": [
        "Create", "Write", "FadeIn", "FadeOut", "Transform", "ReplacementTransform",
        "MoveToTarget", "Indicate", "Flash", "Circumscribe", "ShowPassingFlash",
        "GrowFromCenter", "GrowArrow", "SpinInFromNothing", "Rotate", "Shift",
        "Wait", "AnimationGroup", "LaggedStart", "Succession",
    ],
    "table_matrix": [
        "Table", "MathTable", "IntegerTable", "DecimalTable", "Matrix",
        "IntegerMatrix", "DecimalMatrix",
    ],
    "mobject_ops": [
        "VGroup", "Group", "Mobject", "VMobject",
    ],
}

# Flatten for fast lookup
_KNOWN_OBJECTS: dict[str, str] = {}
for category, objects in MANIM_CATEGORIES.items():
    for obj in objects:
        _KNOWN_OBJECTS[obj] = category


def _classify_error(error_text: str) -> str:
    """Classify a Manim/Python error into a categorical bucket."""
    text = error_text.lower()
    if "attributeerror" in text:
        return "AttributeError"
    if "nameerror" in text or "importerror" in text or "modulenotfounderror" in text:
        return "ImportError_NameError"
    if "typeerror" in text:
        return "TypeError"
    if "syntaxerror" in text or "indentationerror" in text:
        return "SyntaxError"
    if "valueerror" in text:
        return "ValueError"
    if "latex" in text or "tex" in text:
        return "LaTeX_Error"
    if "zero" in text or "zerodivision" in text:
        return "ZeroDivision"
    if "timeout" in text:
        return "Timeout"
    if "index" in text or "key" in text:
        return "IndexError_KeyError"
    return "Other"


def _extract_code_metrics(code: str) -> dict[str, Any]:
    """Analyse the generated Python code and return descriptive metrics."""
    lines = code.splitlines()
    metrics: dict[str, Any] = {
        "total_lines": len(lines),
        "non_empty_lines": sum(1 for l in lines if l.strip()),
        "manim_objects_used": [],
        "object_categories": [],
        "has_3d": False,
        "has_tex": False,
        "has_animation_group": False,
        "lambda_count": 0,
        "self_play_calls": 0,
        "self_wait_calls": 0,
    }

    for obj, category in _KNOWN_OBJECTS.items():
        if re.search(rf"\b{obj}\b", code):
            metrics["manim_objects_used"].append(obj)
            if category not in metrics["object_categories"]:
                metrics["object_categories"].append(category)

    metrics["has_3d"] = "three_d" in metrics["object_categories"]
    metrics["has_tex"] = (
        "MathTex" in code or "Tex(" in code or r"\\" in code
    )
    metrics["has_animation_group"] = "AnimationGroup" in code or "LaggedStart" in code
    metrics["lambda_count"] = code.count("lambda")
    metrics["self_play_calls"] = code.count("self.play(")
    metrics["self_wait_calls"] = code.count("self.wait(")

    # AST parse for deeper metrics
    try:
        tree = ast.parse(code)
        funcs = [n for n in ast.walk(tree) if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))]
        classes = [n for n in ast.walk(tree) if isinstance(n, ast.ClassDef)]
        metrics["function_count"] = len(funcs)
        metrics["class_count"] = len(classes)
        metrics["ast_parse_ok"] = True
    except SyntaxError:
        metrics["function_count"] = 0
        metrics["class_count"] = 0
        metrics["ast_parse_ok"] = False

    return metrics


# ── Session state ─────────────────────────────────────────────────────────────

class PipelineLogger:
    """Tracks a single code generation session and writes structured logs."""

    def __init__(self, prompt: str):
        LOG_DIR.mkdir(exist_ok=True)
        SESSION_DIR.mkdir(exist_ok=True)

        self.session_id = datetime.utcnow().strftime("%Y%m%d_%H%M%S_%f")
        self.prompt = prompt
        self.session_start = time.time()

        self.session: dict[str, Any] = {
            "session_id": self.session_id,
            "timestamp_utc": datetime.utcnow().isoformat(),
            "prompt": prompt,
            "rag_used": False,
            "rag_retrieval": None,
            "generation_attempts": [],
            "final_outcome": "pending",
            "total_duration_s": None,
        }

    # ── RAG events ─────────────────────────────────────────────────────────

    def log_rag_retrieval(self, raw_results: list[dict], context_chars: int):
        """Record what was retrieved from ChromaDB."""
        by_type: dict[str, int] = {}
        sim_scores = []
        for r in raw_results:
            t = r.get("source_type", "unknown")
            by_type[t] = by_type.get(t, 0) + 1
            sim_scores.append(r.get("similarity", 0.0))

        self.session["rag_used"] = True
        self.session["rag_retrieval"] = {
            "chunks_retrieved": len(raw_results),
            "context_chars": context_chars,
            "by_source_type": by_type,
            "similarity_scores": sim_scores,
            "mean_similarity": round(sum(sim_scores) / len(sim_scores), 4) if sim_scores else 0.0,
            "min_similarity": round(min(sim_scores), 4) if sim_scores else 0.0,
            "max_similarity": round(max(sim_scores), 4) if sim_scores else 0.0,
        }

    # ── Code generation events ────────────────────────────────────────────

    def log_generation(
        self,
        attempt: int,
        code: str,
        syntax_ok: bool,
        scene_class_ok: bool,
        render_success: bool | None,
        error_output: str = "",
        latency_s: float = 0.0,
        is_heal_attempt: bool = False,
    ):
        """Log a single code generation or self-healing attempt."""
        code_metrics = _extract_code_metrics(code) if code else {}
        error_category = _classify_error(error_output) if error_output else None

        attempt_record = {
            "attempt_number": attempt,
            "is_heal_attempt": is_heal_attempt,
            "latency_s": round(latency_s, 3),
            "code_metrics": code_metrics,
            "validation": {
                "syntax_ok": syntax_ok,
                "scene_class_ok": scene_class_ok,
            },
            "render_success": render_success,
            "error_category": error_category,
            "error_snippet": error_output[:300] if error_output else None,
        }
        self.session["generation_attempts"].append(attempt_record)

    # ── Final outcome ─────────────────────────────────────────────────────

    def finalize(self, success: bool):
        """Called when the pipeline finishes (success or failure)."""
        self.session["final_outcome"] = "success" if success else "failure"
        self.session["total_duration_s"] = round(time.time() - self.session_start, 3)
        self.session["total_attempts"] = len(self.session["generation_attempts"])
        self.session["heal_attempts"] = sum(
            1 for a in self.session["generation_attempts"] if a["is_heal_attempt"]
        )

        # Write session JSON
        session_file = SESSION_DIR / f"session_{self.session_id}.json"
        session_file.write_text(json.dumps(self.session, indent=2), encoding="utf-8")

        # Append to JSONL log
        with JSONL_PATH.open("a", encoding="utf-8") as f:
            f.write(json.dumps(self.session) + "\n")

    # ── Summary helper ────────────────────────────────────────────────────

    def summary_dict(self) -> dict:
        return {
            "session_id": self.session_id,
            "outcome": self.session.get("final_outcome"),
            "attempts": self.session.get("total_attempts"),
            "rag_chunks": (
                self.session["rag_retrieval"]["chunks_retrieved"]
                if self.session.get("rag_retrieval")
                else 0
            ),
            "duration_s": self.session.get("total_duration_s"),
        }
