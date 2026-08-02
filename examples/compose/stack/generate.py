#!/usr/bin/env python3
import json

compose = {
    "cixCompose": 1,
    "name": "stack",
    "children": {
        "web": {
            "item": "stack-web:v1",
            "bind": {"http": "127.0.0.1:8080"},
        },
        "backend": {
            "item": "stack-backend:current",
            "update": "track",
            "env": {"SUFFIX": " via compose"},
        },
        "db": {"item": "stack-db:v1"},
    },
    "edges": {
        "database": {
            "producer": {
                "child": "db",
                "path": "/run/redis",
            },
            "consumers": {"backend": {}},
        },
        "http": {
            "producer": {
                "child": "backend",
                "path": "/run/backend",
            },
            "consumers": {"web": {}},
        },
    },
}

print(json.dumps(compose, indent=2))
