# LyricsStatus (Rust)

LyricsStatus cambia el estado personalizado de Discord, línea por línea, usando la letra sincronizada de la canción que se está reproduciendo en Spotify.

El backend ha sido reescrito completamente en Rust. No requiere Node.js, una cuenta de desarrollador de Spotify, OAuth propio ni cookies: obtiene el acceso de Spotify desde la conexión que ya existe en Discord.

> **Aviso:** el programa utiliza el token de usuario de Discord. Este se guarda únicamente en `settings.json` en tu equipo, pero automatizar una cuenta puede incumplir los términos de Discord. Úsalo bajo tu propia responsabilidad y nunca compartas el token.

## Requisitos

- [Rust](https://rustup.rs/) estable (1.80 o posterior)
- Una cuenta de Discord con Spotify conectado (Ajustes → Conexiones)
- Spotify reproduciendo música en cualquier dispositivo

## Compilar

```bash
cargo build --release
```

El ejecutable se crea en:

- Linux/macOS: `target/release/lyrics-status`
- Windows: `target\release\lyrics-status.exe`

## Ejecutar

```bash
cargo run --release
```

Después abre [http://localhost:8999](http://localhost:8999). El servidor solo escucha en `127.0.0.1`, por lo que el panel no se expone a la red local.

## Configuración

El panel conserva el formato de `settings.json` de la versión TypeScript, incluidos:

- token y UUID local;
- marca de tiempo y etiqueta del estado;
- plantilla avanzada y emoji Unicode o personalizado;
- desfase fijo o cálculo automático por latencia;
- traducción opcional mediante Google Translate;
- comprobación de actualizaciones.

Las plantillas avanzadas admiten `{lyrics}`, sus variantes en mayúsculas/minúsculas y solo letras, `{timestamp}`, `{song_name}`, sus variantes recortadas, y `{song_author}`. El resultado se limita a 128 caracteres Unicode.

## Fuentes de letras

Se consultan en orden hasta encontrar letras sincronizadas:

1. LrcLib
2. NetEase Music
3. QQ Music

Las respuestas se guardan en `cache/` con nombres SHA-256 seguros. Las canciones repetidas no generan nuevas consultas a los proveedores.

## Arquitectura

- `src/main.rs`: inicialización y tareas asíncronas.
- `src/spotify.rs`: estado de reproducción mediante Discord/Spotify.
- `src/status.rs`: sincronización, plantillas, emojis y estado de Discord.
- `src/sources/`: proveedores de letras y analizador LRC compartido.
- `src/lyrics_fetcher.rs`: fallback entre proveedores y caché.
- `src/server.rs`: panel HTTP y configuración por WebSocket.
- `src/settings.rs`: configuración compatible y persistencia.
- `src/translation.rs`: traducción y caché en memoria.

## Pruebas

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Solución de problemas

- **No cambia el estado:** comprueba el token en el panel y confirma que Spotify está conectado a Discord.
- **No detecta reproducción:** debe existir un dispositivo activo y la canción no debe estar pausada.
- **No encuentra letras:** la canción puede no tener letras sincronizadas en ninguno de los tres proveedores.
- **El puerto está ocupado:** cierra el proceso que usa `127.0.0.1:8999` antes de iniciar LyricsStatus.
