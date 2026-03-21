# Reasonance Dokumentation

## 1. Einfuehrung

### Was ist Reasonance
Reasonance ist eine leichtgewichtige Desktop-IDE fuer Vibecoder — Entwickler, die mit KI-gestuetzten Programmierassistenten arbeiten. Sie bietet eine saubere, fokussierte Umgebung mit integrierten LLM-Terminals, einem Code-Editor und Dateiverwaltung.

### Systemanforderungen
- Linux (KDE Plasma empfohlen), macOS oder Windows
- Mindestens ein LLM-CLI-Tool installiert (Claude Code, Ollama, etc.)

### Installation
Laden Sie die neueste Version von der GitHub Releases-Seite herunter. Unter Linux installieren Sie das .deb- oder .AppImage-Paket.

## 2. Oberflaeche

### Layout
Reasonance verwendet ein Drei-Panel-Layout:
- **Linkes Panel**: Dateibaum zur Projektnavigation
- **Mittleres Panel**: Code-Editor mit Syntaxhervorhebung
- **Rechtes Panel**: LLM-Terminal fuer KI-gestuetztes Programmieren

### Menuleiste
Greifen Sie ueber die Menuleiste auf alle Funktionen zu:
- **Datei**: Ordner oeffnen, Dateien verwalten, letzte Projekte
- **Bearbeiten**: Rueckgaengig, Wiederherstellen, Zwischenablage, Suche
- **Ansicht**: Thema, Lesbarkeit, Panel-Sichtbarkeit
- **Terminal**: LLM-Terminals erstellen, YOLO-Modus
- **Git**: Status, Commit, Push, Pull, Log
- **Hilfe**: Dokumentation, Tastenkuerzel

### Statusleiste
Die untere Statusleiste zeigt:
- App-Name und Anzahl erkannter LLMs
- Aktive Terminal-Sitzungsinfo (Kontext %, Modell, Reset-Timer, Nachrichten)
- Aktive Dateiinfo (Name, Sprache, Kodierung)
- YOLO-Modus-Anzeige (rote Leiste wenn aktiv)

### Tastenkuerzel
| Tastenkuerzel | Aktion |
|---------------|--------|
| Ctrl+P | Schnelle Dateisuche |
| Ctrl+Shift+F | In Dateien suchen |
| Ctrl+S | Datei speichern |
| Ctrl+, | Einstellungen oeffnen |
| F1 | Dokumentation oeffnen |

## 3. Dateiverwaltung

### Ein Projekt oeffnen
Verwenden Sie **Datei > Ordner oeffnen** oder klicken Sie auf "Ordner oeffnen" auf dem Willkommensbildschirm. Letzte Projekte werden fuer schnellen Zugriff aufgelistet.

### Dateien navigieren
Klicken Sie auf Dateien im Dateibaum, um sie zu oeffnen. Rechtsklick fuer Kontextmenu-Aktionen. Verwenden Sie Ctrl+P fuer schnelle Dateisuche nach Name.

### Dateien bearbeiten
Dateien oeffnen sich standardmaessig im Nur-Lesen-Modus. Klicken Sie auf "Nur-Lesen", um den Bearbeitungsmodus umzuschalten. Aenderungen werden mit Schattenkopien fuer die Diff-Erkennung verfolgt.

### Suche
- **Ctrl+P**: Dateien nach Name suchen
- **Ctrl+Shift+F**: Dateiinhalte durchsuchen (grep)

## 4. LLM-Terminal

### Ein LLM starten
Klicken Sie auf die **+**-Schaltflaeche im Terminal-Panel, um verfuegbare LLMs zu sehen. Reasonance erkennt automatisch installierte CLI-Tools (Claude Code, Ollama, etc.).

### Mehrere Instanzen
Fuehren Sie mehrere LLM-Sitzungen gleichzeitig aus. Jede Instanz hat ihren eigenen Tab. Wechseln Sie zwischen Instanzen ueber die Tab-Leiste.

### YOLO-Modus
Aktivieren Sie den YOLO-Modus ueber die Symbolleiste oder **Terminal > YOLO-Modus**. Dies uebergibt das Flag --dangerously-skip-permissions an Claude Code, sodass es ohne Bestaetigungsaufforderungen ausgefuehrt werden kann. Die Statusleiste wird als Warnung rot.

### Kontext-Tracking
Die Statusleiste zeigt die Echtzeit-Nutzung des Kontextfensters, geparst aus der LLM-Ausgabe, einschliesslich:
- Sitzungsnutzung in Prozent mit visueller Leiste
- Aktiver Modellname
- Verbleibende Nachrichten
- Reset-Countdown-Timer

## 5. Git-Integration

Greifen Sie auf Git-Befehle ueber das **Git**-Menu zu. Befehle werden im aktiven Terminal ausgefuehrt:
- **Status**: Arbeitsbaum-Status anzeigen
- **Commit**: Einen Commit starten (Nachricht eingeben)
- **Push**: Zum Remote pushen
- **Pull**: Vom Remote pullen
- **Log**: Letzte Commit-Historie anzeigen

## 6. Einstellungen

Oeffnen Sie die Einstellungen mit **Ctrl+,** oder dem Zahnrad-Symbol.

### Thema
Waehlen Sie zwischen Hell, Dunkel oder System (folgt der OS-Praeferenz). Unter KDE/Wayland verwendet der System-Modus native Erkennung mit Fallback auf Dunkel.

### Sprache
Waehlen Sie aus 9 Sprachen: English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. Arabisch aktiviert RTL-Layout.

### Schrift und Lesbarkeit
- Benutzerdefinierte Schriftfamilie und -groesse
- Modus fuer verbesserte Lesbarkeit: groesserer Text, erhoehter Abstand, optimiert fuer Barrierefreiheit

### LLM-Konfiguration
LLMs werden beim ersten Start automatisch erkannt. Manuelle Konfiguration ueber TOML-Konfigurationsdatei fuer erweiterte Setups.

## 7. Fehlerbehebung

### LLMs nicht erkannt
- Stellen Sie sicher, dass das LLM-CLI-Tool installiert und in Ihrem PATH ist
- Versuchen Sie **Terminal > LLMs erkennen** fuer einen erneuten Scan
- Pruefen Sie die Konfigurationsdatei fuer manuelle Konfiguration

### Unscharfes Rendering unter Linux
Reasonance enthaelt eine Korrektur fuer fraktionale Skalierung unter KDE/Wayland (WebKitGTK). Wenn das Rendering immer noch unscharf ist, pruefen Sie Ihre Display-Skalierungseinstellungen.

### Thema wechselt nicht
Wenn das Thema nicht auf Systemanenderungen reagiert, versuchen Sie es explizit auf Hell oder Dunkel in den Einstellungen zu setzen und dann zurueck auf System.

### FAQ
**F: Kann ich mehrere LLMs gleichzeitig verwenden?**
A: Ja, jedes LLM bekommt seinen eigenen Tab. Klicken Sie auf +, um weitere Instanzen hinzuzufuegen.

**F: Wie konfiguriere ich ein benutzerdefiniertes LLM?**
A: Bearbeiten Sie die TOML-Konfigurationsdatei unter ~/.config/reasonance/config.toml

**F: Funktioniert der YOLO-Modus mit allen LLMs?**
A: Der YOLO-Modus ist derzeit fuer Claude Code optimiert. Andere LLMs koennen andere Bestaetigungsmechanismen haben.
