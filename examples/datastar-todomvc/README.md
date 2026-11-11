# Datastar TodoMVC

> **Note:** The video walkthrough was recorded with Datastar v1.0.0-RC.5 and is
> out of date. The example itself works with Datastar v1.0.5.

TodoMVC implemented with Datastar frontend and [Nushell](https://www.nushell.sh)
/ [cross.stream](https://cablehead.github.io/xs/) backend.

## Live Demo

🚀 **[Try it live](https://datastar-todomvc.cross.stream)**

## Resources

- 📖
  **[Tutorial: Building TodoMVC with Datastar + xs](https://cablehead.github.io/xs/tutorials/datastar-todomvc/)**
- Based on: https://data-star.dev/examples/todomvc

## Requirements

- https://github.com/mitsuhiko/minijinja
- https://github.com/cablehead/http-nu

## Run

```bash
cat serve.nu | http-nu :3001 -
```

Visit http://localhost:3001

## API

- `GET /` - Static files
- `GET /todos/updates` - SSE stream of todo state
- `POST /todos/add` - Add todo (`{"input": "text"}`)
- `POST /todos/{id}/toggle` - Toggle todo completion
- `POST /todos/toggle-all` - Toggle all todos
- `DELETE /todos/{id}` - Delete todo
- `DELETE /todos/completed` - Clear completed todos

## Tests

```bash
nu -c 'source projection_test.nu; __test_all'
```
