"""THIRD model definition."""
import pydantic
from .test2 import MySecondModel


class NotModel:
    def __init__(self, id_):
        self.id = id_


class MyThirdModel(MySecondModel):
    id: pydantic.StrictInt
    name: str

    @pydantic.validator("id")
    def check_id(cls, v: pydantic.StrictInt) -> pydantic.StrictInt:
        if v < 1:
            raise ValueError("`id` must be positive, non-zero value.")

    def _increase_id(self) -> None:
        self.id += 1
