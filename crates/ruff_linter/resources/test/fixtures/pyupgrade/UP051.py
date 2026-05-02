from __future__ import annotations
from typing import Annotated

x: Annotated[int, "metadata"]
y: Annotated[str, 123]

z: Annotated[float, "a", "b", "c"]

a: Annotated[Annotated[int, "inner"], "outer"]

b: Annotated[list[int], "list"]
c: Annotated[dict[str, int], "dict"]

from typing_extensions import Annotated as AnnotatedExt
d: AnnotatedExt[int, "ext"]

def f(arg: Annotated[int, "arg"]):
    pass

TypeAlias = Annotated[int, "alias"]

e: Annotated[int, "quoted 'metadata'"]

f: Annotated[int, {"key": "value"}]

g: Annotated["int", "metadata"]
h: Annotated["List[int]", 123]

i: Annotated[int | str, "metadata"]
j: Annotated[int | float | None, "m1", "m2"]

k: Annotated[list[int] | dict[str, int], "m"]

l: list[Annotated[int, "metadata"]]
m: dict[str, Annotated[float, "m"]]

n: int | Annotated[float, "metadata"]
o: Annotated[int, "m1"] | Annotated[str, "m2"]
