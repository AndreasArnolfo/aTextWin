<div align="center">

<img src="src-tauri/icons/icon.png" width="120" alt="aTextWin logo" />

# aTextWin

**Text expansion pour Windows — l'équivalent d'aText, en plus rapide.**

Créez des abréviations qui se développent automatiquement en texte complet, dans n'importe quelle application.

</div>

---

## ✨ Fonctionnalités

- ⚡ **Expansion instantanée** — un hook clavier global détecte vos abréviations et les remplace en temps réel, dans n'importe quelle app Windows
- 🧩 **Variables dynamiques** — insérez `{date}`, `{heure}`, `{datetime}`, `{clipboard}` ou positionnez le curseur avec `{curseur}`
- 📁 **Groupes & dossiers** — organisez vos snippets par catégorie, filtrez-les en un clic
- 📊 **Statistiques d'usage** — nombre d'expansions et de caractères économisés
- 🛡️ **Confirmation de suppression** — plus de snippet supprimé par erreur
- ⚙️ **Paramètres avancés** — limite de mot (pas d'expansion en plein milieu d'un mot), liste noire d'applications
- 📤 **Import / Export** — sauvegardez et partagez vos snippets au format JSON
- ⌨️ **Raccourci global** — affichez/masquez la fenêtre depuis n'importe où avec `Ctrl+Shift+Espace`
- 🗂️ **System tray & démarrage automatique** — toujours actif en arrière-plan, sans encombrer votre barre des tâches

## 🖥️ Installation

Téléchargez la dernière version depuis les [Releases](../../releases) :

- **Installateur MSI** : `atextwin_x.x.x_x64_en-US.msi`
- **Installateur NSIS** : `atextwin_x.x.x_x64-setup.exe`

## 🚀 Utilisation

1. Ouvrez aTextWin et créez un snippet : une **abréviation** (ex. `;sig`) et son **expansion** (ex. votre signature email)
2. Tapez l'abréviation dans n'importe quelle application — elle se transforme automatiquement en texte complet
3. Ajoutez des variables dynamiques pour des expansions encore plus puissantes :

| Variable | Résultat |
|---|---|
| `{date}` | Date du jour |
| `{heure}` | Heure actuelle |
| `{datetime}` | Date et heure |
| `{clipboard}` | Contenu du presse-papiers |
| `{curseur}` | Position du curseur après l'expansion |

## 🛠️ Stack technique

- **Frontend** : React 18 + TypeScript + Vite
- **Backend** : Rust via [Tauri 2](https://tauri.app/)
- **Hook clavier global** : [`rdev`](https://github.com/Narsil/rdev)
- **Injection de texte** : [`enigo`](https://github.com/enigo-rs/enigo)

## 👨‍💻 Développement

```bash
# Installer les dépendances
npm install

# Lancer en mode développement
npm run tauri dev

# Compiler la version release
npm run tauri build
```

## 📄 Licence

Projet personnel — tous droits réservés.
