# Documentacion de Reasonance

## 1. Introduccion

### Que es Reasonance
Reasonance es un IDE de escritorio ligero disenado para vibecoders — desarrolladores que trabajan con asistentes de codificacion basados en IA. Proporciona un entorno limpio y enfocado con terminales LLM integrados, un editor de codigo y gestion de archivos.

### Requisitos del Sistema
- Linux (KDE Plasma recomendado), macOS o Windows
- Al menos una herramienta CLI de LLM instalada (Claude Code, Ollama, etc.)

### Instalacion
Descarga la ultima version desde la pagina de GitHub Releases. En Linux, instala el paquete .deb o .AppImage.

## 2. Interfaz

### Disposicion
Reasonance utiliza una disposicion de tres paneles:
- **Panel izquierdo**: Arbol de archivos para navegar tu proyecto
- **Panel central**: Editor de codigo con resaltado de sintaxis
- **Panel derecho**: Terminal LLM para codificacion asistida por IA

### Barra de Menu
Accede a todas las funciones desde la barra de menu:
- **Archivo**: Abrir carpetas, gestionar archivos, proyectos recientes
- **Editar**: Deshacer, rehacer, portapapeles, buscar
- **Ver**: Tema, legibilidad, visibilidad de paneles
- **Terminal**: Crear terminales LLM, modo YOLO
- **Git**: Estado, commit, push, pull, log
- **Ayuda**: Documentacion, atajos de teclado

### Barra de Estado
La barra de estado inferior muestra:
- Nombre de la app y cantidad de LLMs detectados
- Info de sesion del terminal activo (contexto %, modelo, temporizador de reset, mensajes)
- Info del archivo activo (nombre, lenguaje, codificacion)
- Indicador de modo YOLO (barra roja cuando esta activo)

### Atajos de Teclado
| Atajo | Accion |
|-------|--------|
| Ctrl+P | Busqueda rapida de archivos |
| Ctrl+Shift+F | Buscar en archivos |
| Ctrl+S | Guardar archivo |
| Ctrl+, | Abrir configuracion |
| F1 | Abrir documentacion |

## 3. Gestion de Archivos

### Abrir un Proyecto
Usa **Archivo > Abrir Carpeta** o haz clic en "Abrir Carpeta" en la pantalla de bienvenida. Los proyectos recientes se listan para acceso rapido.

### Navegar Archivos
Haz clic en los archivos del arbol de archivos para abrirlos. Clic derecho para acciones del menu contextual. Usa Ctrl+P para busqueda rapida de archivos por nombre.

### Editar Archivos
Los archivos se abren en modo solo lectura por defecto. Haz clic en "Solo lectura" para alternar el modo de edicion. Los cambios se rastrean con copias sombra para la deteccion de diferencias.

### Busqueda
- **Ctrl+P**: Buscar archivos por nombre
- **Ctrl+Shift+F**: Buscar en el contenido de archivos (grep)

## 4. Terminal LLM

### Iniciar un LLM
Haz clic en el boton **+** en el panel de terminal para ver los LLMs disponibles. Reasonance detecta automaticamente las herramientas CLI instaladas (Claude Code, Ollama, etc.).

### Multiples Instancias
Ejecuta multiples sesiones LLM simultaneamente. Cada instancia tiene su propia pestana. Cambia entre instancias usando la barra de pestanas.

### Modo YOLO
Activa el modo YOLO desde la barra de herramientas o **Terminal > Modo YOLO**. Esto pasa el flag --dangerously-skip-permissions a Claude Code, permitiendole ejecutar sin solicitudes de confirmacion. La barra de estado se vuelve roja como advertencia.

### Seguimiento de Contexto
La barra de estado muestra el uso de la ventana de contexto en tiempo real, analizado desde la salida del LLM, incluyendo:
- Porcentaje de uso de sesion con barra visual
- Nombre del modelo activo
- Mensajes restantes
- Temporizador de cuenta regresiva para reset

## 5. Integracion Git

Accede a los comandos Git desde el menu **Git**. Los comandos se ejecutan en el terminal activo:
- **Estado**: Mostrar estado del arbol de trabajo
- **Commit**: Iniciar un commit (escribe tu mensaje)
- **Push**: Push al remoto
- **Pull**: Pull del remoto
- **Log**: Mostrar historial de commits recientes

## 6. Configuracion

Abre la configuracion con **Ctrl+,** o el icono de engranaje.

### Tema
Elige entre Claro, Oscuro o Sistema (sigue la preferencia del SO). En KDE/Wayland, el modo Sistema usa deteccion nativa con fallback a oscuro.

### Idioma
Selecciona entre 9 idiomas: English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. El arabe habilita disposicion RTL.

### Fuente y Legibilidad
- Familia y tamano de fuente personalizables
- Modo de Legibilidad Mejorada: texto mas grande, espaciado aumentado, optimizado para accesibilidad

### Configuracion de LLM
Los LLMs se detectan automaticamente en el primer inicio. Configuracion manual mediante archivo de configuracion TOML para configuraciones avanzadas.

## 7. Solucion de Problemas

### LLMs No Detectados
- Asegurate de que la herramienta CLI de LLM este instalada y en tu PATH
- Prueba **Terminal > Detectar LLMs** para re-escanear
- Revisa el archivo de configuracion para configuracion manual

### Renderizado Borroso en Linux
Reasonance incluye una correccion para escalado fraccional en KDE/Wayland (WebKitGTK). Si el renderizado sigue borroso, revisa la configuracion de escalado de tu pantalla.

### El Tema No Cambia
Si el tema no responde a cambios del sistema, intenta configurarlo explicitamente a Claro u Oscuro en Configuracion, luego vuelve a Sistema.

### FAQ
**P: Puedo usar multiples LLMs a la vez?**
R: Si, cada LLM tiene su propia pestana. Haz clic en + para agregar mas instancias.

**P: Como configuro un LLM personalizado?**
R: Edita el archivo de configuracion TOML en ~/.config/reasonance/config.toml

**P: El modo YOLO funciona con todos los LLMs?**
R: El modo YOLO esta actualmente optimizado para Claude Code. Otros LLMs pueden tener mecanismos de confirmacion diferentes.
