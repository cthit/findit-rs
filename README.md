# findIT

findIT is a service discovery tool for finding various IT division services that are hosted and/or made by students of the Chalmers IT Student Division.

## How it works

The application queries the local Docker socket for running containers that have specific labels, and it also supports manually managed services through the admin panel. It groups all services by category and displays them in a web interface.

## Usage

### For Services

To make a service appear in findIT, add the following labels to your Docker container:

| Label | Description | Required |
|-------|-------------|----------|
| `findit.enable` | Set to `true` to opt-in. | Yes |
| `findit.title` | The name of the service. | Yes |
| `findit.url` | The URL to access the service. | Yes |
| `findit.description` | A brief description of the service. | Yes |
| `findit.category` | The category to group the service under. | Yes |
| `findit.github_url` | URL to the source code. | No |
| `findit.icon` | Icon name from the shared icon library. | No |

### Manual Services

If a service cannot be discovered through Docker, you can add it from the admin panel at `/admin`.

- Manual services support the same fields as Docker labels: title, URL, description, category, optional GitHub URL, and optional icon.
- Icons are selected from the shared icon library managed in the same admin panel.

### Running findIT

#### Prerequisites
- Rust and Cargo
- [Dioxus CLI](https://dioxuslabs.com/learn/0.6/getting_started) (`cargo install dioxus-cli`)
- Docker (with access to the Docker socket)

#### Development
To start the development server:

```bash
dx serve
```

#### Production
To build for production:

```bash
dx build --release
```

The application requires access to `/var/run/docker.sock` to function.
