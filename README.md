# BookReview

## 1) Instalar Rust (macOS con Homebrew)

```bash
# Instalar rustup (gestor oficial de toolchains de Rust) vía Homebrew
brew install rustup-init

# Inicializar e instalar toolchain estable
rustup-init -y

# Cargar el entorno en la sesión actual
source "$HOME/.cargo/env"

# Verificar que quedó OK
rustc --version
cargo --version
```

### Alternativas
- **macOS (Homebrew, sin rustup):**
  ```bash
  brew install rust   # instala rustc/cargo directamente (actualiza con brew upgrade)
  ```
- **Linux / WSL2 (oficial):**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```
- **Windows:** usar **Rustup** para Windows o WSL2 + alguno de los métodos anteriores.

---

## 2) Levantar MongoDB con Docker
1. Tener **Docker Desktop** (en macOS puedes instalarlo con Homebrew):
   ```bash
   brew install --cask docker
   ```
   Abre Docker Desktop al menos una vez.

2. Desde la carpeta del repo:
   ```bash
   docker compose up -d
   docker compose ps
   # (opcional) ver logs de Mongo
   docker compose logs -f
   ```

Esto inicia Mongo en **localhost:27017** y persiste datos en un volumen.

---

## 3) Variables de entorno (`.env`)
Crea un archivo **`.env`** en la **raíz** del repo con:

```env
# Conexión a Mongo local (publicado por docker compose)
MONGO_URI=mongodb://localhost:27017

# Nombre de la base de datos (dev)
DB_NAME=bookreview_dev
```


---

## 4) Correr la app
Desde la raíz del proyecto:
```bash
cargo run --bin bookreview         
```

Deberías ver algo como:
```
Rocket has launched from http://127.0.0.1:8000
```

Abre la vista en el navegador:
```
http://127.0.0.1:8000/authors
```


- Si el puerto 8000 está ocupado:
```bash
ROCKET_PORT=8001 cargo run
# Abre http://127.0.0.1:8001/authors
```
