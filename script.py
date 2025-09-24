#!/usr/bin/env python3
"""Convert a combined OpenAPI-style YAML into sectioned Markdown tables.

Usage:
  script.py --input - --output - [--no-sort]

Behavior:
  - Reads YAML from a file or stdin (use '-' for stdin)
  - Writes Markdown to a file or stdout (use '-' for stdout)
  - Does not hardcode any specific input path

This implements the rules in plan_script.md for table layout and sorting.
"""
from __future__ import annotations

import argparse
import sys
import typing as t

try:
    import yaml
except Exception as exc:  # pragma: no cover - helpful error message at runtime
    print(
        "Missing dependency: PyYAML is required. Install with `pip install pyyaml`.",
        file=sys.stderr,
    )
    raise


METHOD_ORDER = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]


def bool_to_yesno(val: t.Any) -> str:
    return "Yes" if bool(val) else "No"


def sanitize_code_span(value: str) -> str:
    """Return a string safe to wrap in single backticks by replacing any literal backticks.

    We replace the ASCII backtick ` with the PRIME character U+2032 (\u2032) to avoid
    breaking Markdown code spans coming from data.
    """
    if value is None:
        return ""
    return value.replace("`", "\u2032")


def normalize_locations(loc: t.Union[str, t.List[str], None]) -> t.Optional[str]:
    if loc is None:
        return None
    if isinstance(loc, str):
        return sanitize_code_span(loc)
    if isinstance(loc, (list, tuple)):
        parts: t.List[str] = []
        for p in loc:
            if p is None:
                continue
            parts.append(sanitize_code_span(str(p)))
        return ", ".join(parts) if parts else None
    # Unexpected type
    return sanitize_code_span(str(loc))


def method_sort_key(method: str) -> t.Tuple[int, str]:
    m = method.upper()
    if m in METHOD_ORDER:
        return (METHOD_ORDER.index(m), "")
    # after known ones, sort lexicographically
    return (len(METHOD_ORDER), m)


def parse_yaml(stream: str) -> t.Dict[str, t.Any]:
    data = yaml.safe_load(stream)
    if not isinstance(data, dict):
        raise ValueError("Top-level YAML must be a mapping of path -> metadata")
    return data


def build_rows(yaml_map: t.Dict[str, t.Any]) -> t.Dict[str, t.List[t.Dict[str, t.Any]]]:
    sections: t.Dict[str, t.List[t.Dict[str, t.Any]]] = {}
    for path, meta in yaml_map.items():
        try:
            endpoint_section = meta.get("section") if isinstance(meta, dict) else None
        except Exception:
            endpoint_section = None
        section_name = endpoint_section or "Uncategorized"

        absolute = False
        owner = None
        endpoint_name = None
        backend_class = None
        methods = {}

        if isinstance(meta, dict):
            absolute = bool(meta.get("absolute", False))
            owner = meta.get("owner")
            endpoint_name = meta.get("name")
            backend_class = meta.get("class")
            methods = meta.get("methods") or {}

        if not isinstance(methods, dict):
            print(
                f"Warning: skipping path {path!r} because 'methods' is not a mapping",
                file=sys.stderr,
            )
            continue

        for method_raw, method_meta in methods.items():
            if method_raw is None:
                continue
            method = str(method_raw).upper()
            if not isinstance(method_meta, dict):
                print(
                    f"Warning: skipping {path!r} {method} because method metadata is not a mapping",
                    file=sys.stderr,
                )
                continue

            region_aware = bool(method_meta.get("region_aware", False))
            notes = method_meta.get("notes") or ""
            loc = method_meta.get("locations")
            locations_joined = normalize_locations(loc)

            row = {
                "path": path,
                "method": method,
                "region_aware": region_aware,
                "absolute": absolute,
                "owner": owner,
                "endpoint_name": endpoint_name,
                "backend_class": backend_class,
                "locations": locations_joined,
                "notes": notes,
            }

            sections.setdefault(section_name, []).append(row)

    return sections


def render_section(section: str, rows: t.List[t.Dict[str, t.Any]]) -> str:
    # Sort rows: by path lexicographic, then by method order
    rows_sorted = sorted(rows, key=lambda r: (r["path"], method_sort_key(r["method"])))

    lines: t.List[str] = []
    lines.append(f"### {section}")
    lines.append("")
    lines.append(
        "| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |"
    )
    lines.append(
        "| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |"
    )

    for r in rows_sorted:
        path_cell = f"`{sanitize_code_span(r['path'])}`"
        method_cell = f"`{sanitize_code_span(r['method'])}`"
        region_cell = bool_to_yesno(r.get("region_aware", False))
        absolute_cell = bool_to_yesno(r.get("absolute", False))
        owner_cell = f"`{sanitize_code_span(r['owner'])}`" if r.get("owner") else ""
        endpoint_cell = (
            f"`{sanitize_code_span(r['endpoint_name'])}`"
            if r.get("endpoint_name")
            else ""
        )
        class_cell = (
            f"`{sanitize_code_span(r['backend_class'])}`"
            if r.get("backend_class")
            else ""
        )
        locations_raw = r.get("locations")
        locations_cell = f"`{locations_raw}`" if locations_raw else ""
        notes_cell = r.get("notes") or ""

        line = f"| {path_cell} | {method_cell} | {region_cell} | {absolute_cell} | {owner_cell} | {endpoint_cell} | {class_cell} | {locations_cell} | {notes_cell} |"
        lines.append(line)

    return "\n".join(lines)


def main(argv: t.Optional[t.List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate Markdown tables from a combined YAML of endpoints"
    )
    parser.add_argument(
        "--input", "-i", default="-", help="Input YAML file (use - for stdin)"
    )
    parser.add_argument(
        "--output", "-o", default="-", help="Output Markdown file (use - for stdout)"
    )
    parser.add_argument(
        "--no-sort",
        dest="no_sort",
        action="store_true",
        help="Do not sort rows; preserve YAML grouping and order where possible",
    )

    args = parser.parse_args(argv)

    # Read input
    try:
        if args.input == "-":
            stream = sys.stdin.read()
        else:
            with open(args.input, "r", encoding="utf-8") as fh:
                stream = fh.read()
    except Exception as exc:
        print(f"Failed to read input {args.input!r}: {exc}", file=sys.stderr)
        return 2

    try:
        yaml_map = parse_yaml(stream)
    except Exception as exc:
        print(f"Failed to parse YAML input: {exc}", file=sys.stderr)
        return 3

    sections = build_rows(yaml_map)

    # Determine section order
    section_names = list(sections.keys())
    if not args.no_sort:
        section_names = sorted(section_names)

    out_parts: t.List[str] = []
    first = True
    for sec in section_names:
        if not first:
            out_parts.append("")
        first = False
        sec_md = render_section(sec, sections[sec])
        out_parts.append(sec_md)

    output_text = "\n\n".join(out_parts) + "\n"

    try:
        if args.output == "-":
            sys.stdout.write(output_text)
        else:
            with open(args.output, "w", encoding="utf-8") as fh:
                fh.write(output_text)
    except Exception as exc:
        print(f"Failed to write output {args.output!r}: {exc}", file=sys.stderr)
        return 4

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
