"""SECOND model definition."""
from dataclasses import dataclass

import pydantic


class MySecondModel(pydantic.BaseModel):
    """This
    is
    a
    multiline
    comment!
    """
    id: pydantic.StrictInt
    name: str

    @pydantic.validator("id")
    def check_id(cls, v: pydantic.StrictInt) -> pydantic.StrictInt:
        if v < 1:
            raise ValueError("`id` must be positive, non-zero value.")
        return v

    @property
    def key(self) -> str:
        return f"{self.id}{self.name}"


@dataclass
class MyDataclass:
    id: int
    name: str
