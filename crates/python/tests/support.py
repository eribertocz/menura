"""Fixtures shared by the tests of the Python package: the conformance model `slice`."""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
MODELS = ROOT / "conformance" / "models"
SLICE = MODELS / "slice"

QUERY = json.dumps(
    {
        "contract": "menura.query",
        "contract_version": "1.1.0",
        "time_zone": "UTC",
        "query": {
            "dimensions": ["sales.region"],
            "metrics": [{"metric": "total_amount", "alias": "total_amount"}],
        },
    }
)


def slice_file(name):
    """The bytes of a file of the model `slice`."""
    return (SLICE / name).read_bytes()


def slice_handle():
    """The handle of the model `slice`, loaded with the default options."""
    import menura

    return menura.Handle.load(slice_file("model.yaml"), slice_file("catalog-schema.json"))


def analyst():
    """The security context `analyst-north.json` of the model `slice`."""
    return slice_file("contexts/analyst-north.json")
