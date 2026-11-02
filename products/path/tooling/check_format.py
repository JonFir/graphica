#!/usr/bin/env python3
"""Check Path's draft format artifacts, not graph semantics or execution."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

try:
    from jsonschema import Draft202012Validator
    import yaml
except ImportError:
    sys.exit("Install tooling/requirements.txt in a virtual environment first.")


ROOT = Path(__file__).resolve().parents[1]
NODE_TYPES = {
    "decision-tree": {"question", "result"},
    "behavior-tree": {"sequence", "selector", "action", "condition"},
    "activity-diagram": {"initial", "action", "decision", "merge", "fork", "join", "final"},
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def unique_mapping(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(isinstance(key, str), "Mapping keys must be strings")
        require(key not in result, f"Duplicate mapping key: {key}")
        result[key] = value
    return result


class DiagramLoader(yaml.SafeLoader):
    """Safe YAML decoding with duplicate and merge-key rejection."""


def construct_mapping(loader: DiagramLoader, node: yaml.MappingNode) -> dict:
    require(isinstance(node, yaml.MappingNode), "Expected a mapping")
    pairs = []
    for key_node, value_node in node.value:
        require(key_node.value != "<<", "YAML merge keys are unsupported")
        pairs.append((loader.construct_object(key_node), loader.construct_object(value_node)))
    return unique_mapping(pairs)


DiagramLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, construct_mapping)


def reject_constant(value: str) -> None:
    raise ValueError(f"Non-JSON numeric constant: {value}")


def decode_json(text: str) -> object:
    return json.loads(text, object_pairs_hook=unique_mapping, parse_constant=reject_constant)


def require_string_scalars(value: object) -> None:
    if isinstance(value, dict):
        for child in value.values():
            require_string_scalars(child)
    elif isinstance(value, list):
        for child in value:
            require_string_scalars(child)
    else:
        require(isinstance(value, str), "Diagram scalar values must be strings; quote YAML scalars")


def decode_diagram(text: str, encoding: str) -> object:
    if encoding == "json":
        value = decode_json(text)
    else:
        for token in yaml.scan(text):
            require(
                not isinstance(token, (yaml.AliasToken, yaml.AnchorToken, yaml.TagToken)),
                "YAML aliases, anchors, and explicit tags are unsupported",
            )
        value = yaml.load(text, Loader=DiagramLoader)
    require_string_scalars(value)
    return value


def check_encoding() -> int:
    invalid = [
        ("json", '{"id":"first","id":"second"}'),
        ("json", '{"nodes":[{"id":"first","id":"second"}]}'),
        ("json", '{"id":NaN}'),
        ("yaml", "id: first\nid: second\n"),
        ("yaml", "nodes:\n  - id: first\n    id: second\n"),
        ("yaml", "id: first\n---\nid: second\n"),
        ("yaml", "id: &anchor first\n"),
        ("yaml", "id: *anchor\n"),
        ("yaml", "id: !!str first\n"),
        ("yaml", "<<: {id: first}\n"),
        ("yaml", "1: first\n"),
        ("yaml", "formatVersion: 0.1\n"),
        ("yaml", "title: true\n"),
        ("yaml", "title: null\n"),
        ("yaml", "title: 2026-11-02\n"),
    ]
    for encoding, text in invalid:
        try:
            decode_diagram(text, encoding)
        except (ValueError, yaml.YAMLError):
            continue
        raise ValueError(f"Accepted invalid {encoding} encoding: {text!r}")
    require(
        decode_diagram('title: "true"\ndescription: "2026-11-02"\n', "yaml")
        == {"title": "true", "description": "2026-11-02"},
        "Quoted YAML strings must retain their values",
    )
    return len(invalid) + 1


def check_examples(validator: Draft202012Validator) -> int:
    directory = ROOT / "examples"
    json_names = {path.stem for path in directory.glob("*.json")}
    yaml_names = {path.stem for path in directory.glob("*.yaml")}
    require(json_names == yaml_names, "Every example must have one YAML and one JSON file")
    require(bool(json_names), "Missing examples")
    coverage = {kind: set() for kind in NODE_TYPES}
    for name in sorted(json_names):
        documents = []
        for encoding in ("json", "yaml"):
            path = directory / f"{name}.{encoding}"
            document = decode_diagram(path.read_text(encoding="utf-8"), encoding)
            errors = list(validator.iter_errors(document))
            require(not errors, f"{path.relative_to(ROOT)}: {errors[0].message if errors else ''}")
            documents.append(document)
        require(documents[0] == documents[1], f"YAML/JSON mismatch: {name}")
        document = documents[0]
        coverage[document["kind"]].update(node["type"] for node in document["nodes"])
    for kind, expected in NODE_TYPES.items():
        require(coverage[kind] == expected, f"Missing node coverage for {kind}: {expected - coverage[kind]}")
    return len(json_names)


def check_fixtures(validator: Draft202012Validator) -> int:
    cases = decode_json((ROOT / "format/schema-cases.json").read_text(encoding="utf-8"))
    require(bool(cases), "Missing schema fixtures")
    names = set()
    for case in cases:
        require(case["name"] not in names, f"Duplicate fixture name: {case['name']}")
        names.add(case["name"])
        require(
            validator.is_valid(case["document"]) == case["valid"],
            f"Unexpected schema result: {case['name']}",
        )
    return len(cases)


def check_links() -> int:
    count = 0
    for path in sorted(ROOT.rglob("*.md")):
        if any(part.startswith(".") for part in path.relative_to(ROOT).parts):
            continue
        text = path.read_text(encoding="utf-8")
        for target in re.findall(r"\[[^\]]*\]\(([^\s)]+)\)", text):
            parsed = urlsplit(target)
            if parsed.scheme or not parsed.path:
                continue
            destination = path.parent / unquote(parsed.path)
            require(destination.exists(), f"Broken link in {path.relative_to(ROOT)}: {target}")
            count += 1
    return count


def main() -> None:
    schema = decode_json((ROOT / "format/path.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    pairs = check_examples(validator)
    cases = check_fixtures(validator)
    encodings = check_encoding()
    links = check_links()
    print(f"PASS: schema, {pairs} example pairs, {cases} schema fixtures, {encodings} encoding cases, {links} local links")
    print("Graph validation and runtime acceptance scenarios are specified, not implemented.")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        sys.exit(f"FAIL: {error}")
