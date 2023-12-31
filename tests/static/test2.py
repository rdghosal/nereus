"""THIRD model definition."""
import enum
import pydantic
from .test2 import MySecondModel


class NotModel:
    def __init__(self, id_):
        self.id = id_


class MyThirdModel(MySecondModel):
    default_value: str
    id: pydantic.StrictInt = pydantic.Field(default=1)
    name: str = "hello"

    @pydantic.validator("id")
    def check_id(cls, v: pydantic.StrictInt) -> pydantic.StrictInt:
        if v < 1:
            raise ValueError("`id` must be positive, non-zero value.")

    def _increase_id(self) -> None:
        self.id += 1

class TestEnum(enum.Enum):
    RED
    BLUE
    GREEN
