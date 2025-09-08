# BookReview

## Quick Start - Docker Compose Options

This project provides multiple Docker Compose configurations for different deployment scenarios:

| File | Description | Services | Access URL | Use Case |
|------|-------------|----------|------------|----------|
| `docker-compose.basic.yml` | **Application + Database** | Web + MongoDB | `http://localhost:8000` | Development, Testing |
| `docker-compose.proxy.yml` | **Application + Database + Reverse Proxy** | Apache + Web + MongoDB | `http://app.localhost` | Production |
| `docker-compose.yml` | **Default (same as proxy)** | Apache + Web + MongoDB | `http://app.localhost` | Production |
| `docker-compose.dev.yml` | **Legacy Development** | Web + MongoDB | `http://localhost:8000` | Backward compatibility |

### Quick Commands

```bash
# Basic setup (recommended for development)
docker compose -f docker-compose.basic.yml up -d --build

# Production setup with reverse proxy
docker compose up -d --build  # Uses docker-compose.yml (proxy setup)
```

---

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

## 4) Docker Compose Deployment Options

The project provides multiple Docker Compose configurations for different deployment scenarios:

### 4.1. Application + Database (Basic Setup)

**File:** `docker-compose.basic.yml`  
**Use case:** Development, testing, simple deployments  
**Services:** Web application + MongoDB  
**Static files:** Served directly by the Rust application  

```bash
# Start services
docker compose -f docker-compose.basic.yml up -d --build

# View logs
docker compose -f docker-compose.basic.yml logs -f web

# Stop services
docker compose -f docker-compose.basic.yml down
```

**Access:** `http://localhost:8000`

### 4.2. Application + Database + Reverse Proxy (Production Setup)

**File:** `docker-compose.proxy.yml` or `docker-compose.yml` (default)  
**Use case:** Production deployments, load balancing, SSL termination  
**Services:** Apache reverse proxy + Web application + MongoDB  
**Static files:** Served by Apache reverse proxy  

```bash
# Start services (using default compose file)
docker compose up -d --build

# Or explicitly use proxy file
docker compose -f docker-compose.proxy.yml up -d --build

# View logs
docker compose logs -f apache
docker compose logs -f web

# Stop services
docker compose down
```

**Important:** Add this line to your `/etc/hosts` file:
```
127.0.0.1 app.localhost
```

**Access:** `http://app.localhost`

### 4.3. Legacy Development Mode

**File:** `docker-compose.dev.yml`  
**Note:** This file is maintained for backward compatibility. Use `docker-compose.basic.yml` for new projects.

```bash
docker compose -f docker-compose.dev.yml up -d --build
```

### 4.4. Seeder (Load Sample Data)

Load sample data into the database for any deployment:

```bash
# For basic setup
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# For proxy setup (default)
docker compose run --rm web sh -lc '/app/seeder'

# For legacy dev setup
docker compose -f docker-compose.dev.yml run --rm web sh -lc '/app/seeder'
```

### 4.5. Testing Image Uploads

1. Go to `/upload` page in the application
2. Upload book covers and author images
3. Test static file access:
   - **Basic setup**: `http://localhost:8000/static/filename`
   - **Proxy setup**: `http://app.localhost/static/filename`

---

## 5) Deployment Comparison

| Feature | Basic Setup | Proxy Setup |
|---------|-------------|-------------|
| **File** | `docker-compose.basic.yml` | `docker-compose.proxy.yml` |
| **Services** | Web + MongoDB | Apache + Web + MongoDB |
| **Static Files** | Served by Rust app | Served by Apache |
| **URL** | `http://localhost:8000` | `http://app.localhost` |
| **Use Case** | Development, Testing | Production, Load Balancing |
| **SSL Support** | Manual setup required | Easy Apache config |
| **Performance** | Good for dev | Better for production |
| **Complexity** | Simple | Medium |

---

## 6) Architecture Overview

### Deployment Architectures

The application supports two main deployment architectures:

**Basic Architecture (Application + Database):**
```
[Client] → [Rust Web App:8000] → [MongoDB:27017]
                ↓
         [Static Files Served by App]
```

**Proxy Architecture (Application + Database + Reverse Proxy):**
```
[Client] → [Apache:80] → [Rust Web App:8000] → [MongoDB:27017]
              ↓
       [Static Files Served by Apache]
```

### Configuration Differences

**Basic Setup (`docker-compose.basic.yml`):**
- Application serves static files directly
- `SERVE_STATIC_FILES=true`
- Direct access via `http://localhost:8000`
- Simpler setup, ideal for development

**Proxy Setup (`docker-compose.proxy.yml`):**
- Apache serves static files
- Application focuses on dynamic content
- `SERVE_STATIC_FILES=false`
- Access via `http://app.localhost`
- Better for production (caching, SSL, load balancing)

### Image Upload System

- **Supported formats**: JPG, JPEG, PNG, GIF, WebP
- **Storage**: Configurable directory via `UPLOADS_DIR`
- **Organization**: Automatic categorization by type (book covers, author images)
- **Unique naming**: UUID-based filenames to prevent conflicts

### Environment Variables

| Variable | Description | Basic Setup | Proxy Setup |
|----------|-------------|-------------|-------------|
| `SERVE_STATIC_FILES` | Whether app serves static files | `true` | `false` |
| `UPLOADS_DIR` | Directory for uploaded files | `/app/uploads` | `/app/uploads` |
| `MONGO_URI` | MongoDB connection string | `mongodb://mongo:27017` | `mongodb://mongo:27017` |
| `DB_NAME` | Database name | `bookreview_dev` | `bookreview_dev` |

---

## 7) API Endpoints

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

## 8) Docker Commands Reference

### Build and Run Commands

```bash
# Basic setup (Application + Database)
docker compose -f docker-compose.basic.yml up -d --build

# Proxy setup (Application + Database + Reverse Proxy)
docker compose -f docker-compose.proxy.yml up -d --build
# or using default file
docker compose up -d --build

# Legacy development setup
docker compose -f docker-compose.dev.yml up -d --build
```

### View Logs

```bash
# Basic setup
docker compose -f docker-compose.basic.yml logs -f web
docker compose -f docker-compose.basic.yml logs -f mongo

# Proxy setup
docker compose logs -f apache
docker compose logs -f web
docker compose logs -f mongo

# All services at once
docker compose logs -f
```

### Stop Services

```bash
# Basic setup
docker compose -f docker-compose.basic.yml down

# Proxy setup (default)
docker compose down

# Stop and remove volumes (reset database)
docker compose down -v
```

### Reset and Rebuild

```bash
# Complete reset with fresh build (basic)
docker compose -f docker-compose.basic.yml down -v
docker compose -f docker-compose.basic.yml up -d --build

# Complete reset with fresh build (proxy)
docker compose down -v
docker compose up -d --build
```

### Seeder Commands

```bash
# Load sample data - basic setup
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# Load sample data - proxy setup
docker compose run --rm web sh -lc '/app/seeder'
```

---

## 9) Testing Different Deployments

### Testing Basic Setup

```bash
# Start basic setup
docker compose -f docker-compose.basic.yml up -d --build

# Test application
curl http://localhost:8000/health

# Test static file serving (upload a file first via web interface)
curl -I http://localhost:8000/static/your-uploaded-file.jpg
# Should show Rocket/Rust headers

# Load sample data
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# Stop
docker compose -f docker-compose.basic.yml down
```

### Testing Proxy Setup

```bash
# Add to /etc/hosts first
echo "127.0.0.1 app.localhost" | sudo tee -a /etc/hosts

# Start proxy setup
docker compose up -d --build

# Test application through proxy
curl http://app.localhost/health

# Test static file serving through Apache
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers

# Load sample data
docker compose run --rm web sh -lc '/app/seeder'

# Stop
docker compose down
```

### Comparing Both Setups

```bash
# Start basic setup on port 8000
docker compose -f docker-compose.basic.yml up -d --build

# In another terminal, start proxy setup on port 80
docker compose up -d --build

# Now you can compare:
# Basic: http://localhost:8000
# Proxy: http://app.localhost

# Don't forget to stop both when done
docker compose -f docker-compose.basic.yml down
docker compose down
```

---

## 10) Testing the Reverse Proxy

### 1. Setup hosts file
Add to `/etc/hosts`:
```
127.0.0.1 app.localhost
```

### 2. Start proxy setup
```bash
docker compose up -d --build
# or explicitly
docker compose -f docker-compose.proxy.yml up -d --build
```

### 3. Test static file serving
```bash
# Upload a test image via the web interface at:
http://app.localhost/upload

# Verify Apache serves static files by checking headers:
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers (Server: Apache/2.4.x)
```

### 4. Test application routing
```bash
# Health check through proxy
curl http://app.localhost/health

# API endpoints through proxy  
curl http://app.localhost/authors
curl http://app.localhost/books
```

### 5. Compare with basic setup
```bash
# Stop proxy setup
docker compose down

# Start basic setup
docker compose -f docker-compose.basic.yml up -d --build

# Test direct access
curl http://localhost:8000/health
curl -I http://localhost:8000/static/your-file.jpg
# Should show Rocket headers (Server: Rocket)
```

---

## 11) Ejecutar la app en **Kubernetes**

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

## 13) Load Testing

The project includes comprehensive JMeter load tests to compare performance between deployment modes.

### Quick Load Testing

**Prerequisites:**
```bash
# Install JMeter (macOS)
brew install jmeter

# Verify installation
jmeter -v
```

**Run Simple Tests:**
```bash
cd load-testing

# Test basic deployment (app + database)
docker compose -f ../docker-compose.basic.yml up -d --build
./simple-test.sh basic 10  # 10 concurrent users

# Test proxy deployment (apache + app + database)  
docker compose -f ../docker-compose.yml up -d --build
./simple-test.sh proxy 10  # 10 concurrent users
```

**Collect System Metrics:**
```bash
# Monitor container resource usage during tests
./collect-metrics.sh basic 300   # Monitor for 5 minutes
./collect-metrics.sh proxy 300   # Monitor for 5 minutes
```

### Complete Test Suite

Run all test scenarios (1, 10, 100, 1000, 5000 users × 5 minutes each):

```bash
cd load-testing
./run-load-tests.sh  # Takes ~50 minutes total
```

### Test Results

Results are saved in `load-testing/results/`:
- **JMeter Results**: `.jtl` files with response times, throughput, error rates
- **HTML Reports**: Interactive dashboards in `*_report/` directories  
- **System Metrics**: CSV files with CPU, memory, network, disk usage
- **Summary Report**: `load_test_report.md` with comparison analysis

### Metrics Collected

**Application Performance:**
- Response times (min, max, average, percentiles)
- Throughput (requests per second)
- Error rates and HTTP status codes
- Request distribution across endpoints

**System Resources:**
- CPU usage percentage per container
- Memory usage (MB and percentage)
- Network I/O (received/transmitted data)
- Disk I/O (read/write operations)
- Process counts

### Analysis Framework

Compare the two deployment architectures across:

1. **Response Times**: Which deployment responds faster?
2. **Throughput**: Which can handle more requests per second?
3. **Resource Efficiency**: Which uses fewer system resources?
4. **Scalability**: Which handles high load better?
5. **Error Patterns**: Which deployment is more stable under stress?

For detailed instructions, see [`load-testing/README.md`](load-testing/README.md).

---

## 14) Troubleshooting

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

**5. Port conflicts**
- Basic setup uses port 8000: ensure it's not in use
- Proxy setup uses port 80: ensure it's not in use
- Check running processes: `lsof -i :8000` or `lsof -i :80`

### Debug Commands

```bash
# Check container status for different setups
docker compose -f docker-compose.basic.yml ps
docker compose ps

# View all logs for basic setup
docker compose -f docker-compose.basic.yml logs

# View all logs for proxy setup
docker compose logs

# Inspect uploads volume
docker volume inspect bookreview_uploads_data

# Test file upload via curl (basic setup)
curl -X POST -F "file=@test.jpg" -F "upload_type=book_cover" -F "entity_id=test" http://localhost:8000/upload

# Test file upload via curl (proxy setup)
curl -X POST -F "file=@test.jpg" -F "upload_type=book_cover" -F "entity_id=test" http://app.localhost/upload

# Test static file serving
curl -I http://localhost:8000/static/test_book_cover_*.jpg  # Basic setup
curl -I http://app.localhost/static/test_book_cover_*.jpg   # Proxy setup
```

### Quick Setup Verification

```bash
# Verify basic setup is working
docker compose -f docker-compose.basic.yml up -d --build
curl http://localhost:8000/health
# Should return: {"status": "healthy"}

# Verify proxy setup is working
docker compose up -d --build
curl http://app.localhost/health
# Should return: {"status": "healthy"}
```