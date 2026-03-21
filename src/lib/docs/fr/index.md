# Documentation de Reasonance

## 1. Introduction

### Qu'est-ce que Reasonance
Reasonance est un IDE de bureau leger concu pour les vibecoders — des developpeurs qui travaillent avec des assistants de codage bases sur l'IA. Il fournit un environnement propre et concentre avec des terminaux LLM integres, un editeur de code et une gestion de fichiers.

### Configuration Requise
- Linux (KDE Plasma recommande), macOS ou Windows
- Au moins un outil CLI LLM installe (Claude Code, Ollama, etc.)

### Installation
Telechargez la derniere version depuis la page GitHub Releases. Sur Linux, installez le paquet .deb ou .AppImage.

## 2. Interface

### Disposition
Reasonance utilise une disposition a trois panneaux :
- **Panneau gauche** : Arborescence de fichiers pour naviguer dans votre projet
- **Panneau central** : Editeur de code avec coloration syntaxique
- **Panneau droit** : Terminal LLM pour le codage assiste par IA

### Barre de Menu
Accedez a toutes les fonctionnalites depuis la barre de menu :
- **Fichier** : Ouvrir des dossiers, gerer les fichiers, projets recents
- **Edition** : Annuler, retablir, presse-papiers, recherche
- **Affichage** : Theme, lisibilite, visibilite des panneaux
- **Terminal** : Creer des terminaux LLM, mode YOLO
- **Git** : Statut, commit, push, pull, log
- **Aide** : Documentation, raccourcis clavier

### Barre d'Etat
La barre d'etat inferieure affiche :
- Nom de l'app et nombre de LLMs detectes
- Info de session du terminal actif (contexte %, modele, minuterie de reinitialisation, messages)
- Info du fichier actif (nom, langage, encodage)
- Indicateur de mode YOLO (barre rouge lorsqu'actif)

### Raccourcis Clavier
| Raccourci | Action |
|-----------|--------|
| Ctrl+P | Recherche rapide de fichiers |
| Ctrl+Shift+F | Rechercher dans les fichiers |
| Ctrl+S | Enregistrer le fichier |
| Ctrl+, | Ouvrir les parametres |
| F1 | Ouvrir la documentation |

## 3. Gestion des Fichiers

### Ouvrir un Projet
Utilisez **Fichier > Ouvrir un Dossier** ou cliquez sur "Ouvrir un Dossier" sur l'ecran d'accueil. Les projets recents sont listes pour un acces rapide.

### Naviguer dans les Fichiers
Cliquez sur les fichiers dans l'arborescence pour les ouvrir. Clic droit pour les actions du menu contextuel. Utilisez Ctrl+P pour une recherche rapide de fichier par nom.

### Editer des Fichiers
Les fichiers s'ouvrent en mode lecture seule par defaut. Cliquez sur "Lecture seule" pour basculer en mode edition. Les modifications sont suivies avec des copies fantomes pour la detection des differences.

### Recherche
- **Ctrl+P** : Rechercher des fichiers par nom
- **Ctrl+Shift+F** : Rechercher dans le contenu des fichiers (grep)

## 4. Terminal LLM

### Demarrer un LLM
Cliquez sur le bouton **+** dans le panneau terminal pour voir les LLMs disponibles. Reasonance detecte automatiquement les outils CLI installes (Claude Code, Ollama, etc.).

### Instances Multiples
Executez plusieurs sessions LLM simultanement. Chaque instance a son propre onglet. Basculez entre les instances en utilisant la barre d'onglets.

### Mode YOLO
Activez le mode YOLO depuis la barre d'outils ou **Terminal > Mode YOLO**. Cela transmet le flag --dangerously-skip-permissions a Claude Code, lui permettant de s'executer sans invites de confirmation. La barre d'etat devient rouge en avertissement.

### Suivi du Contexte
La barre d'etat affiche l'utilisation de la fenetre de contexte en temps reel, analysee a partir de la sortie du LLM, incluant :
- Pourcentage d'utilisation de session avec barre visuelle
- Nom du modele actif
- Messages restants
- Minuterie de compte a rebours pour la reinitialisation

## 5. Integration Git

Accedez aux commandes Git depuis le menu **Git**. Les commandes s'executent dans le terminal actif :
- **Statut** : Afficher l'etat de l'arbre de travail
- **Commit** : Demarrer un commit (saisissez votre message)
- **Push** : Pousser vers le distant
- **Pull** : Tirer depuis le distant
- **Log** : Afficher l'historique des commits recents

## 6. Parametres

Ouvrez les parametres avec **Ctrl+,** ou l'icone d'engrenage.

### Theme
Choisissez entre Clair, Sombre ou Systeme (suit la preference du SE). Sur KDE/Wayland, le mode Systeme utilise la detection native avec repli sur Sombre.

### Langue
Selectionnez parmi 9 langues : English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. L'arabe active la disposition RTL.

### Police et Lisibilite
- Famille et taille de police personnalisables
- Mode de Lisibilite Amelioree : texte plus grand, espacement augmente, optimise pour l'accessibilite

### Configuration LLM
Les LLMs sont detectes automatiquement au premier lancement. Configuration manuelle via fichier de configuration TOML pour les configurations avancees.

## 7. Depannage

### LLMs Non Detectes
- Assurez-vous que l'outil CLI LLM est installe et dans votre PATH
- Essayez **Terminal > Detecter les LLMs** pour re-scanner
- Verifiez le fichier de configuration pour la configuration manuelle

### Rendu Flou sur Linux
Reasonance inclut une correction pour la mise a l'echelle fractionnaire sur KDE/Wayland (WebKitGTK). Si le rendu est toujours flou, verifiez vos parametres de mise a l'echelle de l'affichage.

### Le Theme Ne Change Pas
Si le theme ne repond pas aux changements du systeme, essayez de le definir explicitement sur Clair ou Sombre dans les Parametres, puis revenez a Systeme.

### FAQ
**Q : Puis-je utiliser plusieurs LLMs en meme temps ?**
R : Oui, chaque LLM a son propre onglet. Cliquez sur + pour ajouter d'autres instances.

**Q : Comment configurer un LLM personnalise ?**
R : Editez le fichier de configuration TOML a ~/.config/reasonance/config.toml

**Q : Le mode YOLO fonctionne-t-il avec tous les LLMs ?**
R : Le mode YOLO est actuellement optimise pour Claude Code. D'autres LLMs peuvent avoir des mecanismes de confirmation differents.
