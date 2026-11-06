"""Audit an isolated weight edit at source and GGUF stages; no inference or training."""

import argparse
import hashlib
import json
from pathlib import Path
import struct


def compare_tensors(base, edited, expected):
    if base.keys() != edited.keys():
        raise ValueError("tensor names differ")
    changed, unchanged = [], []
    for name in sorted(base):
        for field in ("shape", "dtype", "bytes"):
            if base[name][field] != edited[name][field]:
                raise ValueError(f"tensor layout differs: {name} {field}")
        (changed if base[name]["sha256"] != edited[name]["sha256"] else unchanged).append(name)
    if set(changed) != set(expected):
        raise ValueError(f"changed tensors differ: expected {sorted(expected)}, got {changed}")
    return {"changed": changed, "unchanged": unchanged}


def safetensors_records(path):
    records = {}
    with Path(path).open("rb") as stream:
        raw_length = stream.read(8)
        if len(raw_length) != 8:
            raise ValueError("truncated safetensors header")
        length = struct.unpack("<Q", raw_length)[0]
        if length > 16 * 1024 * 1024:
            raise ValueError("safetensors header exceeds audit limit")
        raw_header = stream.read(length)
        if len(raw_header) != length:
            raise ValueError("truncated safetensors header")
        header = json.loads(raw_header)
        for name, info in header.items():
            if name == "__metadata__":
                continue
            start, end = info["data_offsets"]
            if start < 0 or end < start:
                raise ValueError("invalid tensor offsets")
            stream.seek(8 + length + start)
            remaining, digest = end - start, hashlib.sha256()
            while remaining:
                block = stream.read(min(remaining, 1024 * 1024))
                if not block:
                    raise ValueError(f"truncated tensor: {name}")
                digest.update(block)
                remaining -= len(block)
            records[name] = {"shape": info["shape"], "dtype": info["dtype"], "bytes": end - start, "sha256": digest.hexdigest()}
    return records


def gguf_records(path):
    from gguf import GGUFReader

    reader = GGUFReader(path)
    records = {
        tensor.name: {"shape": tensor.shape.tolist(), "dtype": int(tensor.tensor_type), "bytes": tensor.data.nbytes, "sha256": hashlib.sha256(memoryview(tensor.data).cast("B")).hexdigest()}
        for tensor in reader.tensors
    }
    metadata = {}
    for name, field in reader.fields.items():
        digest = hashlib.sha256()
        digest.update(json.dumps([int(kind) for kind in field.types]).encode())
        for part in field.parts:
            digest.update(part.dtype.str.encode())
            digest.update(json.dumps(list(part.shape)).encode())
            digest.update(memoryview(part).cast("B"))
        metadata[name] = digest.hexdigest()
    return records, metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("source", "gguf"))
    parser.add_argument("base", type=Path)
    parser.add_argument("edited", type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    if args.stage == "source":
        base, edited = safetensors_records(args.base), safetensors_records(args.edited)
        expected = {tensor["name"] for tensor in manifest["tensors"]}
        for tensor in manifest["tensors"]:
            for records, field in ((base, "base_sha256"), (edited, "edited_sha256")):
                if records[tensor["name"]]["sha256"] != tensor[field]:
                    raise ValueError(f"published tensor hash differs: {tensor['name']} {field}")
        metadata = None
    else:
        base, metadata = gguf_records(args.base)
        edited, edited_metadata = gguf_records(args.edited)
        if metadata != edited_metadata:
            raise ValueError("GGUF metadata differs, including architecture/tokenizer/template")
        expected = {"blk.15.ffn_down.weight", "blk.19.ffn_down.weight"}
    result = {"schema": 1, "stage": args.stage, **compare_tensors(base, edited, expected), "base": base, "edited": edited, "metadata": metadata}
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2)
        stream.write("\n")
    print(f"{args.stage}: {len(base)} tensors, exactly {len(result['changed'])} declared changes; layouts and other payloads identical")


if __name__ == "__main__":
    main()
