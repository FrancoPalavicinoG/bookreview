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

## 4) Correr la app con Docker

### 4.1. Levantar la app y Mongo DB
Desde la raíz del proyecto:

```bash
docker compose up -d --build
docker compose logs -f web   # ver salida de Rocket
```

Deberías ver algo como:
```
Rocket has launched from http://0.0.0.0:8000
```

Comprueba salud:
```bash
curl http://127.0.0.1:8000/health
# ok
```

Abre la web:
```
http://127.0.0.1:8000/
```

---

### 4.2. Seeder
El seeder **no** corre automáticamente. Ejecútalo manualmente:

```bash
docker compose run --rm web sh -lc '/app/seeder'
```
---

### 4.3. Apagar / Reset DB
```bash
# Apagar (mantiene datos en el volumen)
docker compose down

# Apagar y borrar volumen de Mongo (reinicia la base desde cero)
docker compose down -v

```

---
## 5) Ejecutar la app en **Kubernetes**

### Prerrequisitos
- `kubectl` instalado.
- **Cluster local**:
  - **kind** (Kubernetes in Docker).


---

### Usando **kind** (recomendado)
1) **Crear** un cluster local:
```bash
kind create cluster --name bookreview
```

2) **Construir** la imagen de la web (local):
```bash
docker build -t bookreview-web:kind .
```

3) **Cargar** la imagen al cluster kind:
```bash
kind load docker-image bookreview-web:kind --name bookreview
```

4) **Aplicar** los manifiestos:
```bash
kubectl apply -f k8s/bookreview.yaml
```

5) **Ver pods** hasta que estén Ready:
```bash
kubectl -n bookreview get pods -w
```

6) **Exponer** localmente con port-forward y abrir la web:
```bash
kubectl -n bookreview port-forward svc/bookreview-web 8000:8000
# Luego visita: http://127.0.0.1:8000/
```

**Seeder** (carga de datos de ejemplo) en el cluster:
```bash
kubectl -n bookreview run seeder --image=bookreview-web:kind --restart=Never --rm -it --env="MONGO_URI=mongodb://mongo:27017" --env="DB_NAME=bookreview_dev" -- /app/seeder
```

---

### Comandos útiles
- **Logs** de la web:
```bash
kubectl -n bookreview logs deploy/bookreview-web -f
```

- **Reiniciar** la web (rolling restart):
```bash
kubectl -n bookreview rollout restart deploy bookreview-web
kubectl -n bookreview rollout status deploy/bookreview-web
```

- **Actualizar** la imagen tras recompilar (kind):
```bash
docker build -t bookreview-web:kind .
kind load docker-image bookreview-web:kind --name bookreview
kubectl -n bookreview rollout restart deploy bookreview-web
```

- **Eliminar** todo lo desplegado:
```bash
kubectl delete -f k8s/bookreview.yaml
```

- **Borrar** el cluster kind (si usaste kind):
```bash
kind delete cluster --name bookreview
```