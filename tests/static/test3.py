"""THIRD model definition."""
import pydantic


class MyThirdModel(pydantic.BaseModel):
    id: pydantic.StrictInt
    name: str

    @pydantic.validator("id")
    def check_id(cls, v: pydantic.StrictInt) -> pydantic.StrictInt:
        if v < 1:
            raise ValueError("`id` must be positive, non-zero value.")
        return v
