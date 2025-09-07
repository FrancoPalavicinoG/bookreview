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

# Static file serving (true/false)
SERVE_STATIC_FILES=true

# Uploads directory path
UPLOADS_DIR=uploads
```

**Tip:** Puedes usar el archivo `.env.example` como plantilla:
```bash
cp .env.example .env
```

---

## 4) Running the Application

### 4.1. Development Mode (Without Reverse Proxy)

For development, run the application directly serving static files:

```bash
docker compose -f docker-compose.dev.yml up -d --build
docker compose -f docker-compose.dev.yml logs -f web
```

Access the application at:
```
http://127.0.0.1:8000/
```

### 4.2. Production Mode (With Apache Reverse Proxy)

For production with Apache reverse proxy:

```bash
docker compose up -d --build
docker compose logs -f apache
docker compose logs -f web
```

**Important:** Add this line to your `/etc/hosts` file:
```
127.0.0.1 app.localhost
```

Then access the application at:
```
http://app.localhost/
```

### 4.3. Seeder (Load Sample Data)

Load sample data into the database:

```bash
# Development mode
docker compose -f docker-compose.dev.yml run --rm web sh -lc '/app/seeder'

# Production mode  
docker compose run --rm web sh -lc '/app/seeder'
```

### 4.4. Testing Image Uploads

1. Go to `/upload` page in the application
2. Upload book covers and author images
3. Test static file access:
   - **Development mode**: `http://127.0.0.1:8000/static/filename`
   - **Production mode**: `http://app.localhost/static/filename`

---

## 5) Architecture Overview

### Reverse Proxy Setup

The application supports two modes:

**Without Reverse Proxy (Development):**
- Application serves static files directly
- `SERVE_STATIC_FILES=true`
- Access via `http://localhost:8000`

**With Reverse Proxy (Production):**
- Apache serves static files
- Application focuses on dynamic content
- `SERVE_STATIC_FILES=false`
- Access via `http://app.localhost`

### Image Upload System

- **Supported formats**: JPG, JPEG, PNG, GIF, WebP
- **Storage**: Configurable directory via `UPLOADS_DIR`
- **Organization**: Automatic categorization by type (book covers, author images)
- **Unique naming**: UUID-based filenames to prevent conflicts

### Environment Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `SERVE_STATIC_FILES` | Whether app serves static files | `true` | `false` |
| `UPLOADS_DIR` | Directory for uploaded files | `uploads` | `/app/uploads` |
| `MONGO_URI` | MongoDB connection string | - | `mongodb://localhost:27017` |
| `DB_NAME` | Database name | - | `bookreview_dev` |

---

## 6) API Endpoints

### Core Routes
- `GET /` - Home page with dashboard
- `GET /health` - Health check endpoint
- `GET /upload` - Image upload interface
- `POST /upload` - Handle file uploads

### Resource Routes
- `/authors/*` - Author management
- `/books/*` - Book management  
- `/reviews/*` - Review management
- `/sales/*` - Sales data management

### Static Files
- `/static/*` - Static file serving (when not behind proxy)

---

## 7) Docker Commands

### Build and Run
```bash
# Production with reverse proxy
docker compose up -d --build

# Development without reverse proxy
docker compose -f docker-compose.dev.yml up -d --build
```

### View Logs
```bash
# Apache logs
docker compose logs -f apache

# Application logs
docker compose logs -f web

# Database logs
docker compose logs -f mongo
```

### Stop Services
```bash
# Stop all services
docker compose down

# Stop and remove volumes (reset database)
docker compose down -v
```

### Reset and Rebuild
```bash
# Complete reset with fresh build
docker compose down -v
docker compose up -d --build
```

---

## 8) Testing the Reverse Proxy

### 1. Setup hosts file
Add to `/etc/hosts`:
```
127.0.0.1 app.localhost
```

### 2. Start services
```bash
docker compose up -d --build
```

### 3. Test static file serving
```bash
# Upload a test image via the web interface at:
http://app.localhost/upload

# Verify Apache serves static files by checking headers:
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers
```

### 4. Test application routing
```bash
# Health check through proxy
curl http://app.localhost/health

# API endpoints through proxy  
curl http://app.localhost/authors
curl http://app.localhost/books
```

### 5. Compare with development mode
```bash
# Stop production mode
docker compose down

# Start development mode
docker compose -f docker-compose.dev.yml up -d --build

# Test direct access
curl http://localhost:8000/health
curl -I http://localhost:8000/static/your-file.jpg
# Should show Rocket headers
```

---
## 9) Ejecutar la app en **Kubernetes**

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

---

## 10) Troubleshooting

### Common Issues

**1. "app.localhost" not resolving**
- Ensure `/etc/hosts` contains: `127.0.0.1 app.localhost`
- Try clearing DNS cache: `sudo dscacheutil -flushcache` (macOS)

**2. Static files not loading behind proxy**
- Check Apache container logs: `docker compose logs apache`
- Verify uploads volume is mounted correctly
- Ensure file permissions are correct

**3. File upload fails**
- Check uploads directory exists and is writable
- Verify file size limits in Apache configuration
- Check application logs for detailed errors

**4. Database connection issues**
- Ensure MongoDB container is healthy: `docker compose ps`
- Check network connectivity between containers
- Verify environment variables are set correctly

### Debug Commands

```bash
# Check container status
docker compose ps

# View all logs
docker compose logs

# Inspect uploads volume
docker volume inspect bookreview_uploads_data

# Test file upload via curl
curl -X POST -F "file=@test.jpg" -F "upload_type=book_cover" -F "entity_id=test" http://app.localhost/upload

# Test static file serving
curl -I http://app.localhost/static/test_book_cover_*.jpg
```