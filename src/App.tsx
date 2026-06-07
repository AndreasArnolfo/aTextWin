import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { Snippet, Stats, Settings } from "./types";
import "./App.css";

type ModalState =
  | { mode: "closed" }
  | { mode: "add" }
  | { mode: "edit"; snippet: Snippet };

function App() {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [active, setActive] = useState(true);
  const [autostart, setAutostart] = useState(false);
  const [search, setSearch] = useState("");
  const [modal, setModal] = useState<ModalState>({ mode: "closed" });
  const [formAbbr, setFormAbbr] = useState("");
  const [formExpansion, setFormExpansion] = useState("");
  const [formGroup, setFormGroup] = useState("");
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<Snippet | null>(null);
  const [groupFilter, setGroupFilter] = useState("");
  const [stats, setStats] = useState<Stats>({ total_expansions: 0, chars_saved: 0 });
  const [showStats, setShowStats] = useState(false);
  const [settings, setSettings] = useState<Settings>({ require_word_boundary: true, blacklist: [] });
  const [showSettings, setShowSettings] = useState(false);
  const [blacklistInput, setBlacklistInput] = useState("");
  const [ieMessage, setIeMessage] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    const [list, isActive, autostartEnabled, currentStats, currentSettings] = await Promise.all([
      invoke<Snippet[]>("get_snippets"),
      invoke<boolean>("get_active"),
      isAutostartEnabled(),
      invoke<Stats>("get_stats"),
      invoke<Settings>("get_settings"),
    ]);
    setSnippets(list);
    setActive(isActive);
    setAutostart(autostartEnabled);
    setStats(currentStats);
    setSettings(currentSettings);
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const toggleActive = async () => {
    const next = !active;
    await invoke("set_active", { active: next });
    setActive(next);
  };

  const toggleAutostart = async () => {
    const next = !autostart;
    if (next) {
      await enableAutostart();
    } else {
      await disableAutostart();
    }
    setAutostart(next);
  };

  const openAdd = () => {
    setFormAbbr("");
    setFormExpansion("");
    setFormGroup(groupFilter !== "__none__" ? groupFilter : "");
    setModal({ mode: "add" });
  };

  const openEdit = (snippet: Snippet) => {
    setFormAbbr(snippet.abbreviation);
    setFormExpansion(snippet.expansion);
    setFormGroup(snippet.group);
    setModal({ mode: "edit", snippet });
  };

  const closeModal = () => setModal({ mode: "closed" });

  const handleSave = async () => {
    const abbr = formAbbr.trim();
    const expansion = formExpansion;
    const group = formGroup.trim();
    if (!abbr || !expansion.trim()) return;

    setSaving(true);
    try {
      if (modal.mode === "edit") {
        await invoke("update_snippet", {
          id: modal.snippet.id,
          abbreviation: abbr,
          expansion,
          enabled: modal.snippet.enabled,
          group,
        });
      } else {
        await invoke("add_snippet", { abbreviation: abbr, expansion, group });
      }
      await loadData();
      closeModal();
    } finally {
      setSaving(false);
    }
  };

  const openStats = async () => {
    const currentStats = await invoke<Stats>("get_stats");
    setStats(currentStats);
    setShowStats(true);
  };

  const handleResetStats = async () => {
    await invoke("reset_stats");
    setStats({ total_expansions: 0, chars_saved: 0 });
  };

  const handleExport = async () => {
    const path = await saveDialog({
      title: "Exporter les snippets",
      defaultPath: "atextwin-snippets.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    try {
      await invoke("export_snippets", { path });
      setIeMessage(`Export réussi : ${snippets.length} snippet(s) enregistré(s).`);
    } catch (err) {
      setIeMessage(`Échec de l'export : ${err}`);
    }
  };

  const handleImport = async () => {
    const path = await openDialog({
      title: "Importer des snippets",
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    try {
      const count = await invoke<number>("import_snippets", { path });
      await loadData();
      setIeMessage(`Import réussi : ${count} snippet(s) ajouté(s).`);
    } catch (err) {
      setIeMessage(`Échec de l'import : ${err}`);
    }
  };

  const persistSettings = async (next: Settings) => {
    setSettings(next);
    await invoke("update_settings", {
      requireWordBoundary: next.require_word_boundary,
      blacklist: next.blacklist,
    });
  };

  const toggleWordBoundary = () => {
    persistSettings({ ...settings, require_word_boundary: !settings.require_word_boundary });
  };

  const addBlacklistEntry = () => {
    const entry = blacklistInput.trim().toLowerCase();
    if (!entry || settings.blacklist.includes(entry)) {
      setBlacklistInput("");
      return;
    }
    persistSettings({ ...settings, blacklist: [...settings.blacklist, entry] });
    setBlacklistInput("");
  };

  const removeBlacklistEntry = (entry: string) => {
    persistSettings({ ...settings, blacklist: settings.blacklist.filter((b) => b !== entry) });
  };

  const requestDelete = (snippet: Snippet) => setConfirmDelete(snippet);

  const cancelDelete = () => setConfirmDelete(null);

  const confirmDeleteSnippet = async () => {
    if (!confirmDelete) return;
    const id = confirmDelete.id;
    await invoke("delete_snippet", { id });
    setSnippets((prev) => prev.filter((sn) => sn.id !== id));
    setConfirmDelete(null);
  };

  const handleToggleSnippet = async (snippet: Snippet) => {
    await invoke("update_snippet", {
      id: snippet.id,
      abbreviation: snippet.abbreviation,
      expansion: snippet.expansion,
      enabled: !snippet.enabled,
      group: snippet.group,
    });
    setSnippets((prev) =>
      prev.map((sn) =>
        sn.id === snippet.id ? { ...sn, enabled: !sn.enabled } : sn
      )
    );
  };

  const groups = Array.from(
    new Set(snippets.map((sn) => sn.group).filter((g) => g.trim() !== ""))
  ).sort((a, b) => a.localeCompare(b));

  const filtered = snippets.filter((sn) => {
    const matchesSearch =
      sn.abbreviation.toLowerCase().includes(search.toLowerCase()) ||
      sn.expansion.toLowerCase().includes(search.toLowerCase());
    const matchesGroup =
      groupFilter === ""
        ? true
        : groupFilter === "__none__"
        ? sn.group.trim() === ""
        : sn.group === groupFilter;
    return matchesSearch && matchesGroup;
  });

  const enabledCount = snippets.filter((sn) => sn.enabled).length;

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="header-left">
          <span className="logo">aTextWin</span>
          <span className="badge">{enabledCount} actif{enabledCount !== 1 ? "s" : ""}</span>
          <span className="badge" title="Raccourci clavier global pour afficher/masquer la fenêtre, fonctionne depuis n'importe quelle application">
            Ctrl+Maj+Espace pour afficher/masquer
          </span>
        </div>
        <div className="header-right">
          <button className="btn-stats" onClick={openStats} title="Voir les statistiques d'usage">
            Statistiques
          </button>
          <button className="btn-stats" onClick={() => setShowSettings(true)} title="Paramètres avancés">
            Paramètres
          </button>
          <button
            className={`master-toggle ${active ? "on" : "off"}`}
            onClick={toggleActive}
            title={active ? "Désactiver l'expansion" : "Activer l'expansion"}
          >
            <span className="dot" />
            {active ? "Actif" : "Pausé"}
          </button>
        </div>
      </header>

      {/* Toolbar */}
      <div className="toolbar">
        <input
          className="search"
          type="search"
          placeholder="Rechercher…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {groups.length > 0 && (
          <select
            className="group-filter"
            value={groupFilter}
            onChange={(e) => setGroupFilter(e.target.value)}
            title="Filtrer par groupe"
          >
            <option value="">Tous les groupes</option>

            <option value="__none__">Sans groupe</option>
            {groups.map((g) => (
              <option key={g} value={g}>
                {g}
              </option>
            ))}
          </select>
        )}
        <label className="autostart-toggle" title="Lance aTextWin automatiquement (réduit dans le tray) au démarrage de Windows">
          <input
            type="checkbox"
            checked={autostart}
            onChange={toggleAutostart}
          />
          Démarrer avec Windows
        </label>
        <button className="btn-cancel" onClick={handleExport} title="Exporter les snippets en JSON">
          Exporter
        </button>
        <button className="btn-cancel" onClick={handleImport} title="Importer des snippets depuis un JSON">
          Importer
        </button>
        <button className="btn-add" onClick={openAdd}>
          + Nouveau snippet
        </button>
      </div>

      {ieMessage && (
        <div className="ie-banner" onClick={() => setIeMessage(null)}>
          {ieMessage}
        </div>
      )}

      {/* Snippet list */}
      <main className="list">
        {filtered.length === 0 ? (
          <div className="empty">
            {search
              ? "Aucun snippet ne correspond à la recherche."
              : "Aucun snippet. Cliquez sur « Nouveau snippet » pour commencer."}
          </div>
        ) : (
          filtered.map((sn) => (
            <div
              key={sn.id}
              className={`card ${sn.enabled ? "" : "card--disabled"}`}
            >
              <div className="card-body">
                {sn.group.trim() !== "" && (
                  <span className="group-badge">{sn.group}</span>
                )}
                <code className="abbr">{sn.abbreviation}</code>
                <span className="arrow">→</span>
                <span className="expansion">{sn.expansion}</span>
              </div>
              <div className="card-actions">
                <button
                  className={`btn-dot ${sn.enabled ? "btn-dot--on" : "btn-dot--off"}`}
                  onClick={() => handleToggleSnippet(sn)}
                  title={sn.enabled ? "Désactiver" : "Activer"}
                />
                <button className="btn-edit" onClick={() => openEdit(sn)}>
                  Modifier
                </button>
                <button
                  className="btn-delete"
                  onClick={() => requestDelete(sn)}
                >
                  Supprimer
                </button>
              </div>
            </div>
          ))
        )}
      </main>

      {/* Modal */}
      {modal.mode !== "closed" && (
        <div className="overlay" onClick={closeModal}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">
              {modal.mode === "edit" ? "Modifier le snippet" : "Nouveau snippet"}
            </h2>

            <div className="field">
              <label>Abréviation</label>
              <input
                type="text"
                value={formAbbr}
                onChange={(e) => setFormAbbr(e.target.value)}
                placeholder="ex: btw"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                }}
              />
              <span className="hint">
                Tapez ce raccourci n'importe où pour déclencher l'expansion.
              </span>
            </div>

            <div className="field">
              <label>Groupe (optionnel)</label>
              <input
                type="text"
                list="group-options"
                value={formGroup}
                onChange={(e) => setFormGroup(e.target.value)}
                placeholder="ex: Travail, Email…"
              />
              <datalist id="group-options">
                {groups.map((g) => (
                  <option key={g} value={g} />
                ))}
              </datalist>
              <span className="hint">
                Regroupez vos snippets pour les retrouver plus facilement.
              </span>
            </div>

            <div className="field">
              <label>Expansion</label>
              <textarea
                value={formExpansion}
                onChange={(e) => setFormExpansion(e.target.value)}
                placeholder="ex: by the way"
                rows={5}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                    handleSave();
                  }
                }}
              />
              <span className="hint">
                Variables : <code>{"{date}"}</code>, <code>{"{heure}"}</code>,{" "}
                <code>{"{datetime}"}</code>, <code>{"{clipboard}"}</code>,{" "}
                <code>{"{curseur}"}</code> (position du curseur après expansion). Ctrl+Entrée pour sauvegarder.
              </span>
            </div>

            <div className="modal-footer">
              <button className="btn-cancel" onClick={closeModal}>
                Annuler
              </button>
              <button
                className="btn-save"
                onClick={handleSave}
                disabled={!formAbbr.trim() || !formExpansion.trim() || saving}
              >
                {saving ? "Sauvegarde…" : modal.mode === "edit" ? "Sauvegarder" : "Ajouter"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Settings */}
      {showSettings && (
        <div className="overlay" onClick={() => setShowSettings(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">Paramètres avancés</h2>

            <div className="field">
              <label
                className="autostart-toggle"
                style={{ fontSize: 13, color: "var(--text)" }}
              >
                <input
                  type="checkbox"
                  checked={settings.require_word_boundary}
                  onChange={toggleWordBoundary}
                />
                Exiger une limite de mot avant l'abréviation
              </label>
              <span className="hint">
                Si activé, l'expansion ne se déclenche que si l'abréviation est
                précédée d'un espace, d'une ponctuation ou du début du texte
                (évite les déclenchements au milieu d'un mot).
              </span>
            </div>

            <div className="field">
              <label>Liste noire d'applications</label>
              <div className="blacklist-input-row">
                <input
                  type="text"
                  value={blacklistInput}
                  onChange={(e) => setBlacklistInput(e.target.value)}
                  placeholder="ex: keepass.exe"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addBlacklistEntry();
                    }
                  }}
                />
                <button className="btn-cancel" onClick={addBlacklistEntry}>
                  Ajouter
                </button>
              </div>
              <span className="hint">
                Nom de l'exécutable (ex: keepass.exe) des applications où
                l'expansion doit rester désactivée.
              </span>
              {settings.blacklist.length > 0 && (
                <div className="blacklist-items">
                  {settings.blacklist.map((entry) => (
                    <span key={entry} className="blacklist-chip">
                      {entry}
                      <button
                        className="blacklist-chip-remove"
                        onClick={() => removeBlacklistEntry(entry)}
                        title="Retirer de la liste noire"
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              )}
            </div>

            <div className="modal-footer">
              <button className="btn-save" onClick={() => setShowSettings(false)}>
                Fermer
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Stats */}
      {showStats && (
        <div className="overlay" onClick={() => setShowStats(false)}>
          <div className="modal modal--confirm" onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">Statistiques d'usage</h2>
            <div className="stats-grid">
              <div className="stat-card">
                <span className="stat-value">{stats.total_expansions}</span>
                <span className="stat-label">expansions effectuées</span>
              </div>
              <div className="stat-card">
                <span className="stat-value">{stats.chars_saved}</span>
                <span className="stat-label">caractères économisés</span>
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn-cancel" onClick={handleResetStats}>
                Réinitialiser
              </button>
              <button className="btn-save" onClick={() => setShowStats(false)}>
                Fermer
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete confirmation */}
      {confirmDelete && (
        <div className="overlay" onClick={cancelDelete}>
          <div className="modal modal--confirm" onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">Supprimer ce snippet ?</h2>
            <p className="confirm-text">
              <code className="abbr">{confirmDelete.abbreviation}</code> sera
              définitivement supprimé. Cette action est irréversible.
            </p>
            <div className="modal-footer">
              <button className="btn-cancel" onClick={cancelDelete}>
                Annuler
              </button>
              <button className="btn-danger" onClick={confirmDeleteSnippet}>
                Supprimer
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
