import json, re, ast, time
from datetime import datetime
from pathlib import Path
from typing import Any

LOG_DIR = Path(__file__).parent.parent / "logs"
JSONL_PATH = LOG_DIR / "pipeline_log.jsonl"
SESSION_DIR = LOG_DIR / "sessions"

MANIM_CATEGORIES = {
    "geometry": [
        "Circle",
        "Square",
        "Rectangle",
        "Triangle",
        "Polygon",
        "RegularPolygon",
        "Line",
        "Arrow",
        "DoubleArrow",
        "DashedLine",
        "Dot",
        "Cross",
        "Ellipse",
        "Arc",
        "AnnularSector",
        "Sector",
        "Annulus",
        "CurvedArrow",
    ],
    "text": [
        "Text",
        "Tex",
        "MathTex",
        "MarkupText",
        "BraceLabel",
        "Brace",
        "BulletedList",
        "Title",
        "Paragraph",
    ],
    "graphing": [
        "Axes",
        "NumberPlane",
        "PolarPlane",
        "ComplexPlane",
        "NumberLine",
        "BarChart",
        "FunctionGraph",
        "ParametricCurve",
        "ImplicitFunction",
        "ValueTracker",
    ],
    "three_d": [
        "ThreeDScene",
        "Surface",
        "Sphere",
        "Cube",
        "Cone",
        "Cylinder",
        "Torus",
        "Dodecahedron",
        "Arrow3D",
        "Line3D",
    ],
    "animation": [
        "Create",
        "Write",
        "FadeIn",
        "FadeOut",
        "Transform",
        "ReplacementTransform",
        "MoveToTarget",
        "Indicate",
        "Flash",
        "Circumscribe",
        "ShowPassingFlash",
        "GrowFromCenter",
        "GrowArrow",
        "SpinInFromNothing",
        "Rotate",
        "Shift",
        "Wait",
        "AnimationGroup",
        "LaggedStart",
        "Succession",
    ],
    "table_matrix": [
        "Table",
        "MathTable",
        "IntegerTable",
        "DecimalTable",
        "Matrix",
        "IntegerMatrix",
        "DecimalMatrix",
    ],
    "mobject_ops": ["VGroup", "Group", "Mobject", "VMobject"],
}
_KNOWN_OBJECTS = {obj: cat for cat, objs in MANIM_CATEGORIES.items() for obj in objs}


def _classify_error(text):
    t = text.lower()
    for kw, label in [
        ("attributeerror", "AttributeError"),
        ("nameerror", "ImportError_NameError"),
        ("importerror", "ImportError_NameError"),
        ("modulenotfounderror", "ImportError_NameError"),
        ("typeerror", "TypeError"),
        ("syntaxerror", "SyntaxError"),
        ("indentationerror", "SyntaxError"),
        ("valueerror", "ValueError"),
        ("latex", "LaTeX_Error"),
        ("zerodivision", "ZeroDivision"),
        ("timeout", "Timeout"),
        ("indexerror", "IndexError_KeyError"),
        ("keyerror", "IndexError_KeyError"),
    ]:
        if kw in t:
            return label
    return "Other"


def _extract_code_metrics(code):
    lines = code.splitlines()
    m: dict[str, Any] = {
        "total_lines": len(lines),
        "non_empty_lines": sum(1 for l in lines if l.strip()),
        "manim_objects_used": [],
        "object_categories": [],
        "has_3d": False,
        "has_tex": False,
        "has_animation_group": False,
        "lambda_count": code.count("lambda"),
        "self_play_calls": code.count("self.play("),
        "self_wait_calls": code.count("self.wait("),
    }
    for obj, cat in _KNOWN_OBJECTS.items():
        if re.search(rf"\b{obj}\b", code):
            m["manim_objects_used"].append(obj)
            if cat not in m["object_categories"]:
                m["object_categories"].append(cat)
    m["has_3d"] = "three_d" in m["object_categories"]
    m["has_tex"] = "MathTex" in code or "Tex(" in code or r"\\" in code
    m["has_animation_group"] = "AnimationGroup" in code or "LaggedStart" in code
    try:
        tree = ast.parse(code)
        m["function_count"] = sum(
            1
            for n in ast.walk(tree)
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
        )
        m["class_count"] = sum(1 for n in ast.walk(tree) if isinstance(n, ast.ClassDef))
        m["ast_parse_ok"] = True
    except SyntaxError:
        m["function_count"] = m["class_count"] = 0
        m["ast_parse_ok"] = False
    return m


class PipelineLogger:
    def __init__(self, prompt):
        LOG_DIR.mkdir(exist_ok=True)
        SESSION_DIR.mkdir(exist_ok=True)
        self.session_id = datetime.utcnow().strftime("%Y%m%d_%H%M%S_%f")
        self.prompt = prompt
        self.session_start = time.time()
        self.session = {
            "session_id": self.session_id,
            "timestamp_utc": datetime.utcnow().isoformat(),
            "prompt": prompt,
            "rag_used": False,
            "rag_retrieval": None,
            "generation_attempts": [],
            "final_outcome": "pending",
            "total_duration_s": None,
        }

    def log_rag_retrieval(self, raw_results, context_chars):
        by_type = {}
        for r in raw_results:
            by_type[r.get("source_type", "unknown")] = (
                by_type.get(r.get("source_type", "unknown"), 0) + 1
            )
        sims = [r.get("similarity", 0.0) for r in raw_results]
        self.session["rag_used"] = True
        self.session["rag_retrieval"] = {
            "chunks_retrieved": len(raw_results),
            "context_chars": context_chars,
            "by_source_type": by_type,
            "similarity_scores": sims,
            "mean_similarity": round(sum(sims) / len(sims), 4) if sims else 0.0,
            "min_similarity": round(min(sims), 4) if sims else 0.0,
            "max_similarity": round(max(sims), 4) if sims else 0.0,
        }

    def log_generation(
        self,
        attempt,
        code,
        syntax_ok,
        scene_class_ok,
        render_success,
        error_output="",
        latency_s=0.0,
        is_heal_attempt=False,
    ):
        self.session["generation_attempts"].append(
            {
                "attempt_number": attempt,
                "is_heal_attempt": is_heal_attempt,
                "latency_s": round(latency_s, 3),
                "code_metrics": _extract_code_metrics(code) if code else {},
                "validation": {
                    "syntax_ok": syntax_ok,
                    "scene_class_ok": scene_class_ok,
                },
                "render_success": render_success,
                "error_category": (
                    _classify_error(error_output) if error_output else None
                ),
                "error_snippet": error_output[:300] if error_output else None,
            }
        )

    def finalize(self, success):
        self.session["final_outcome"] = "success" if success else "failure"
        self.session["total_duration_s"] = round(time.time() - self.session_start, 3)
        self.session["total_attempts"] = len(self.session["generation_attempts"])
        self.session["heal_attempts"] = sum(
            1 for a in self.session["generation_attempts"] if a["is_heal_attempt"]
        )
        (SESSION_DIR / f"session_{self.session_id}.json").write_text(
            json.dumps(self.session, indent=2), encoding="utf-8"
        )
        with JSONL_PATH.open("a", encoding="utf-8") as f:
            f.write(json.dumps(self.session) + "\n")

    def summary_dict(self):
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
