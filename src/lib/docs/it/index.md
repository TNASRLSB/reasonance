# Documentazione di Reasonance

## 1. Introduzione

### Cos'e Reasonance
Reasonance e un IDE desktop leggero progettato per i vibecoder — sviluppatori che lavorano con assistenti di codifica basati su AI. Offre un ambiente pulito e focalizzato con terminali LLM integrati, un editor di codice e gestione dei file.

### Requisiti di Sistema
- Linux (KDE Plasma consigliato), macOS o Windows
- Almeno uno strumento CLI LLM installato (Claude Code, Ollama, ecc.)

### Installazione
Scarica l'ultima versione dalla pagina GitHub Releases. Su Linux, installa il pacchetto .deb o .AppImage.

## 2. Interfaccia

### Layout
Reasonance utilizza un layout a tre pannelli:
- **Pannello sinistro**: Albero dei file per navigare il progetto
- **Pannello centrale**: Editor di codice con evidenziazione della sintassi
- **Pannello destro**: Terminale LLM per la codifica assistita da AI

### Barra dei Menu
Accedi a tutte le funzionalita dalla barra dei menu:
- **File**: Apri cartelle, gestisci file, progetti recenti
- **Modifica**: Annulla, ripristina, appunti, cerca
- **Visualizza**: Tema, leggibilita, visibilita pannelli
- **Terminale**: Crea terminali LLM, modalita YOLO
- **Git**: Stato, commit, push, pull, log
- **Aiuto**: Documentazione, scorciatoie da tastiera

### Barra di Stato
La barra di stato in basso mostra:
- Nome dell'app e numero di LLM rilevati
- Info sessione terminale attiva (contesto %, modello, timer reset, messaggi)
- Info file attivo (nome, linguaggio, codifica)
- Indicatore modalita YOLO (barra rossa quando attiva)

### Scorciatoie da Tastiera
| Scorciatoia | Azione |
|-------------|--------|
| Ctrl+P | Ricerca rapida file |
| Ctrl+Shift+F | Cerca nei file |
| Ctrl+S | Salva file |
| Ctrl+, | Apri impostazioni |
| F1 | Apri documentazione |

## 3. Gestione File

### Aprire un Progetto
Usa **File > Apri Cartella** o clicca "Apri Cartella" nella schermata di benvenuto. I progetti recenti sono elencati per un accesso rapido.

### Navigare i File
Clicca sui file nell'albero dei file per aprirli. Clicca col tasto destro per le azioni del menu contestuale. Usa Ctrl+P per cercare rapidamente un file per nome.

### Modificare i File
I file si aprono in modalita sola lettura per impostazione predefinita. Clicca "Sola lettura" per attivare la modalita modifica. Le modifiche sono tracciate con copie shadow per il rilevamento delle differenze.

### Ricerca
- **Ctrl+P**: Cerca file per nome
- **Ctrl+Shift+F**: Cerca nel contenuto dei file (grep)

## 4. Terminale LLM

### Avviare un LLM
Clicca il pulsante **+** nel pannello terminale per vedere gli LLM disponibili. Reasonance rileva automaticamente gli strumenti CLI installati (Claude Code, Ollama, ecc.).

### Istanze Multiple
Esegui piu sessioni LLM contemporaneamente. Ogni istanza ha la propria scheda. Passa tra le istanze usando la barra delle schede.

### Modalita YOLO
Abilita la modalita YOLO dalla barra degli strumenti o da **Terminale > Modalita YOLO**. Questo passa il flag --dangerously-skip-permissions a Claude Code, permettendogli di eseguire senza richieste di conferma. La barra di stato diventa rossa come avvertimento.

### Tracciamento del Contesto
La barra di stato mostra l'utilizzo della finestra di contesto in tempo reale, analizzato dall'output dell'LLM, incluso:
- Percentuale di utilizzo della sessione con barra visiva
- Nome del modello attivo
- Messaggi rimanenti
- Timer di countdown per il reset

## 5. Integrazione Git

Accedi ai comandi Git dal menu **Git**. I comandi vengono eseguiti nel terminale attivo:
- **Stato**: Mostra lo stato dell'albero di lavoro
- **Commit**: Avvia un commit (digita il tuo messaggio)
- **Push**: Push al remoto
- **Pull**: Pull dal remoto
- **Log**: Mostra la cronologia dei commit recenti

## 6. Impostazioni

Apri le impostazioni con **Ctrl+,** o l'icona ingranaggio.

### Tema
Scegli tra Chiaro, Scuro o Sistema (segue la preferenza del SO). Su KDE/Wayland, la modalita Sistema usa il rilevamento nativo con fallback su scuro.

### Lingua
Seleziona tra 9 lingue: English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. L'arabo abilita il layout RTL.

### Font e Leggibilita
- Famiglia e dimensione del font personalizzabili
- Modalita Leggibilita Migliorata: testo piu grande, spaziatura aumentata, ottimizzata per l'accessibilita

### Configurazione LLM
Gli LLM vengono rilevati automaticamente al primo avvio. Configurazione manuale tramite file di configurazione TOML per setup avanzati.

## 7. Risoluzione Problemi

### LLM Non Rilevati
- Assicurati che lo strumento CLI LLM sia installato e nel tuo PATH
- Prova **Terminale > Rileva LLM** per una nuova scansione
- Controlla il file di configurazione per la configurazione manuale

### Rendering Sfocato su Linux
Reasonance include una correzione per lo scaling frazionario su KDE/Wayland (WebKitGTK). Se il rendering e ancora sfocato, controlla le impostazioni di scaling del display.

### Il Tema Non Cambia
Se il tema non risponde ai cambiamenti di sistema, prova a impostarlo esplicitamente su Chiaro o Scuro nelle Impostazioni, poi torna a Sistema.

### FAQ
**D: Posso usare piu LLM contemporaneamente?**
R: Si, ogni LLM ha la propria scheda. Clicca + per aggiungere altre istanze.

**D: Come configuro un LLM personalizzato?**
R: Modifica il file di configurazione TOML in ~/.config/reasonance/config.toml

**D: La modalita YOLO funziona con tutti gli LLM?**
R: La modalita YOLO e attualmente ottimizzata per Claude Code. Altri LLM potrebbero avere meccanismi di conferma diversi.
