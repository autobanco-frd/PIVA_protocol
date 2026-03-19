# PIVA Protocol

**Protocolo de Intercambio de Valor Antifrágil** - Infraestructura soberana y descentralizada para digitalización de Activos del Mundo Real (RWA).

## Características

- ✅ **512 MB RAM Optimized** - Diseñado para VPS de bajos recursos
- ✅ **Antifrágil** - Se fortalece bajo estrés y demanda
- ✅ **ISO 20022 Compatible** - Interoperabilidad bancaria
- ✅ **OpenTimestamps** - Anclaje a Bitcoin para valor probatorio
- ✅ **Iroh P2P** - Red moderna con Content-Addressing
- ✅ **Rust Native** - Seguridad y rendimiento sin GC

## Arquitectura

```
piva/
├── piva-core      # Estructuras de datos fundamentales
├── piva-crypto    # Primitivas criptográficas (SHA-3, BLAKE3, Ed25519)
├── piva-storage   # Persistencia con redb (embebida, ACID)
├── piva-net       # Red P2P con Iroh
├── piva-iso       # Adaptador ISO 20022
└── piva-cli       # Interfaz de línea de comandos
```

## Requisitos

- Rust 1.75+
- 512 MB RAM mínimo
- 10 GB disco (para Mainnet)

## Instalación

```bash
# Clonar repositorio
git clone https://github.com/piva-protocol/piva
cd piva

# Compilar versión optimizada
cargo build --release

# Binario optimizado en target/release/piva (~8 MB)
```

## Uso Rápido

```bash
# Inicializar nodo
./target/release/piva init

# Iniciar en Devnet (para desarrollo)
./target/release/piva node --network devnet

# Registrar un diploma
./target/release/piva asset register \
  --file diploma.pdf \
  --type diploma \
  --desc "Título Universitario"

# Verificar integridad
./target/release/piva asset verify piva_dev_abc123...

# Listar assets locales
./target/release/piva asset list
```

## Modos de Red

| Modo | Puerto | Persistencia | Uso |
|------|--------|--------------|-----|
| **Devnet** | 7800 | RAM | Desarrollo y pruebas |
| **Testnet** | 7801 | Disco | Pruebas de red pública |
| **Mainnet** | 7802 | Disco | Producción con valor real |

## Desarrollo

```bash
# Tests completos
cargo test --workspace

# Tests por módulo
cargo test -p piva-crypto
cargo test -p piva-storage

# Verificación de memoria (RSS < 400 MB)
cargo build --release
./target/release/piva node --network devnet &
ps -o rss= -p $!  # Debe ser < 50MB en Devnet
```

## Licencia

MIT OR Apache-2.0

by FrD @autobanco
