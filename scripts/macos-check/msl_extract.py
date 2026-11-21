#!/usr/bin/env python3
"""Extract the RefineUI blur composite from shaders.metal into C++ that compiles
against msl_shim.h, so `clang++ -fsyntax-only` catches typos and type errors in
the MSL on Linux. It is NOT a Metal compiler: Metal-only semantics are not checked."""
import re, sys, pathlib
src = pathlib.Path(sys.argv[1]).read_text()
body = src[src.index("// RefineUI patch: real backdrop blur"):]
pre = body[: body.index("constexpr sampler")]
pre = pre[: pre.index("struct RefineBlurVarying")] + "struct RefineBlurVarying { float4 position; };\n"
code = body[body.index("float refine_corner_radius"):]
for attr in ("[[stage_in]]", "[[buffer(0)]]", "[[texture(0)]]"):
    code = code.replace(attr, "")
swz = lambda s: re.sub(r"\.(xy|zw|yz)\b(?!\()", r".\1()", s)
code = re.sub(r"\.a\b", ".w", re.sub(r"\.rgb\b", ".xyz()", swz(code)))
decl = "float4 refine_blur_sample(texture2d<float> t, constant struct RefineBlurParams *p, float2 position);\n"
print('#include "msl_shim.h"\n' + swz(pre) + decl + code)
