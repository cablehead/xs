# Todo App Example

A simple todo list application demonstrating how to build a web application using cross-stream's [built-in HTTP server](https://cablehead.github.io/xs/reference/http-server/).

## Features

- Add new todos
- Toggle todo completion status 
- Persistent storage using cross-stream's event store

## Structure

- `handler.nu` - Event handler for processing HTTP requests and managing todos
- `index.html` - Frontend interface with styling and JavaScript

## Running the App

1. Start cross-stream with HTTP server enabled:
```bash
xs serve ./store --http :5007
```

2. Load the handler and template:
```nushell
# Install minijinja-cli first: https://github.com/mitsuhiko/minijinja
cat handler.nu | .append todo.handler.register
cat index.html | .append index.html
```

3. Visit: http://localhost:5007 in your browser

## Event Structure

The todo state is rebuilt from two event types:

- `todo` - Contains the text content of new todos
- `todo.toggle` - Records completion status changes

The handler aggregates these events to maintain the current state of all todos, with each todo having a unique ID, text content, and completion status.
