"""FIRST model definition."""
import pydantic


class MyFirstModel(pydantic.BaseModel, SomeOtherClass):
    id: pydantic.StrictInt
    name: str

    def __str__(self) -> str:
        return (
            "MyFirstModel("
                + ", ".join([f"{k}={v!r}" for k, v in self.__dict__.items()])
                + ")"
            )

    # A comment.
    @pydantic.validator("id")
    def check_id(cls, v: pydantic.StrictInt) -> pydantic.StrictInt:
        if v < 1:
            raise ValueError("`id` must be positive, non-zero value.")
        return v

    def get_random_list(self) -> list[int | float]:
        return [1, 1.0, 2]
