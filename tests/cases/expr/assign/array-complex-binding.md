### Source
```js parse:expr
[ a().b ] = [ c ]
```

### Output: minified
```js
[a().b]=[c]
```

### Output: ast
```json
{
  "Assignment": {
    "span": "0:17",
    "operator": "Assign",
    "left": {
      "AssignmentPattern": {
        "Array": {
          "span": "0:9",
          "elements": [
            {
              "span": "2:7",
              "target": {
                "Expr": {
                  "Member": {
                    "span": "2:7",
                    "object": {
                      "Expr": {
                        "Call": {
                          "span": "2:5",
                          "callee": {
                            "Expr": {
                              "IdentRef": {
                                "span": "2:3",
                                "name": "a"
                              }
                            }
                          },
                          "arguments_span": "3:5",
                          "arguments": []
                        }
                      }
                    },
                    "property": {
                      "Ident": {
                        "span": "6:7",
                        "name": "b"
                      }
                    }
                  }
                }
              },
              "initializer": null
            }
          ],
          "rest": null
        }
      }
    },
    "right": {
      "Literal": {
        "span": "12:17",
        "literal": {
          "Array": {
            "elements": [
              {
                "Expr": {
                  "IdentRef": {
                    "span": "14:15",
                    "name": "c"
                  }
                }
              }
            ]
          }
        }
      }
    }
  }
}
```
