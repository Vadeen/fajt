### Source
```js parse:expr
a <<= b
```

### Output: minified
```js
a<<=b
```

### Output: ast
```json
{
  "Assignment": {
    "span": "0:7",
    "operator": "LeftShift",
    "left": {
      "Expr": {
        "IdentRef": {
          "span": "0:1",
          "name": "a"
        }
      }
    },
    "right": {
      "IdentRef": {
        "span": "6:7",
        "name": "b"
      }
    }
  }
}
```
