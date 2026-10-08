#!/usr/bin/env python3
"""Validate frozen work-order metadata and repository scope."""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ID_RE = re.compile(r"^WO-[A-Z0-9]+(?:-[A-Z0-9]+)*$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class ValidationError(ValueError):
    """A work order or changed-path set violates its contract."""


def validate_json_schema(instance: object, schema: dict[str, object]) -> bool:
    """Validate the report schemas' deliberately small JSON Schema subset."""
    if "const" in schema and instance != schema["const"]:
        return False
    if "enum" in schema and instance not in schema["enum"]:
        return False
    expected = schema.get("type")
    if expected:
        types = expected if isinstance(expected, list) else [expected]
        checks = {"object": lambda x: isinstance(x, dict), "array": lambda x: isinstance(x, list),
                  "string": lambda x: isinstance(x, str), "integer": lambda x: type(x) is int,
                  "number": lambda x: type(x) in (int, float), "boolean": lambda x: type(x) is bool,
                  "null": lambda x: x is None}
        if not any(t in checks and checks[t](instance) for t in types):
            return False
    if isinstance(instance, dict):
        required = schema.get("required", [])
        props = schema.get("properties", {})
        if any(key not in instance for key in required) or len(instance) < schema.get("minProperties", 0):
            return False
        if schema.get("additionalProperties") is False and instance.keys() - props.keys():
            return False
        for key, value in instance.items():
            if key in props and not validate_json_schema(value, props[key]):
                return False
            if key not in props and isinstance(schema.get("additionalProperties"), dict):
                if not validate_json_schema(value, schema["additionalProperties"]):
                    return False
    if isinstance(instance, list):
        if len(instance) < schema.get("minItems", 0):
            return False
        if "items" in schema and any(not validate_json_schema(item, schema["items"]) for item in instance):
            return False
    if isinstance(instance, str):
        if len(instance) < schema.get("minLength", 0) or len(instance) > schema.get("maxLength", sys.maxsize):
            return False
        if "pattern" in schema and re.search(schema["pattern"], instance) is None:
            return False
    return True


def load_wo(wo_id: str, root: Path = ROOT) -> tuple[dict[str, str], str]:
    if not ID_RE.fullmatch(wo_id):
        raise ValidationError("invalid WO ID")
    resolved_root = root.resolve()
    path = (resolved_root / "work" / f"{wo_id}.md").resolve()
    if not path.is_relative_to(resolved_root):
        raise ValidationError("work-order path escapes the repository")
    raw = path.read_bytes()
    try:
        content = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValidationError("work order is not UTF-8") from exc
    if not content.startswith("---\n") or "\n---\n" not in content[4:]:
        raise ValidationError("missing front matter")
    boundary = content.find("\n---\n", 4)
    fields: dict[str, str] = {}
    for line in content[4:boundary].splitlines():
        if ":" not in line:
            raise ValidationError("malformed front matter")
        key, value = line.split(":", 1)
        if key in fields:
            raise ValidationError(f"duplicate front matter key: {key}")
        fields[key] = value.strip()
    expected = fields.get("spec_sha256", "")
    frozen = re.sub(r"(?m)^spec_sha256: [0-9a-f]{64}\n", "", content, count=1)
    actual = hashlib.sha256(frozen.encode("utf-8")).hexdigest()
    if not re.fullmatch(r"[0-9a-f]{64}", expected) or expected != actual:
        raise ValidationError("frozen spec SHA-256 mismatch")
    if fields.get("id") != wo_id or not fields.get("revision", "").isdigit():
        raise ValidationError("front matter ID or revision mismatch")
    if not re.fullmatch(r"[0-9a-f]{40}", fields.get("base_sha", "")):
        raise ValidationError("base_sha must be a full lowercase git SHA-1")
    return fields, expected


def changed_path_allowed(path: str, allowed: list[str], protected: list[str]) -> bool:
    from pathlib import PurePosixPath

    candidate = PurePosixPath(path)
    if candidate.is_absolute() or ".." in candidate.parts or "\\" in path:
        return False
    if any(candidate.match(pattern) for pattern in protected):
        return False
    return any(candidate.match(pattern) for pattern in allowed)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wo_id", nargs="?")
    parser.add_argument("--ready", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.self_test:
            fields, spec_hash = load_wo("WO-BOOT-04")
            if fields.get("status") != "READY" or len(spec_hash) != 64:
                raise ValidationError("self-test WO is not frozen READY")
            allow = ["scripts/**"]
            protected = ["scripts/oracle/**"]
            if not changed_path_allowed("scripts/wo_driver.py", allow, protected):
                raise ValidationError("allowlist self-test rejected an allowed path")
            if changed_path_allowed("scripts/oracle/frozen.py", allow, protected):
                raise ValidationError("allowlist self-test accepted a protected path")
            print("frozen work order valid")
        else:
            if not args.wo_id:
                parser.error("wo_id is required unless --self-test is used")
            fields, spec_hash = load_wo(args.wo_id)
            if args.ready and fields.get("status") != "READY":
                raise ValidationError("work order is not READY")
            print(f"valid {args.wo_id} revision {fields['revision']} sha256 {spec_hash}")
    except (OSError, ValidationError) as exc:
        print(f"work order error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
