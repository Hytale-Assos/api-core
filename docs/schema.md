# Schéma de base de données — décisions validées

Dernière mise à jour : 15 septembre 2026

Ce document fige les décisions de modélisation prises lors de la revue du
diagramme, avant écriture de la migration `0002`. Il remplace les choix du
diagramme initial (`Blank diagram - Page 1.csv`).

## 1. Principes transverses

| Sujet | Décision | Raison |
|---|---|---|
| Clé primaire | `UUID v7` sur **toutes** les tables | Ordonné dans le temps → bonne localité d'index (pas de fragmentation comme v4), reste non devinable et uniforme côté Rust (`sqlx::FromRow`). Nécessite la feature `v7` du crate `uuid`. Les anciens `id` v4 de `modules` restent valides (un UUID reste un UUID). |
| Horodatage | `TIMESTAMPTZ` partout | Indépendant du fuseau, cohérent avec `0001_init.sql`. |
| Cycle de vie | `is_enabled BOOLEAN NOT NULL DEFAULT TRUE` **et** `deleted_at TIMESTAMPTZ NULL` | Deux états distincts : désactivation réversible (`is_enabled=false`) vs retrait (`deleted_at`). Non redondants. |
| Suppression | Soft-delete uniquement | Conserve l'historique et la stabilité des FK. Impose de filtrer `deleted_at IS NULL` dans toutes les lectures et des index partiels pour les contraintes d'unicité. |
| `updated_at` | Trigger `set_updated_at()` sur chaque table | Met `NEW.updated_at = now()` à chaque `UPDATE`, de façon automatique et fiable (pas de dépendance au code applicatif). |
| Secrets | `api_key_hash` (HMAC-SHA256 déterministe) + `hmac_secret` (AES-256-GCM) | Inchangé : voir `src/crypto.rs`. Chiffrement applicatif, la base ne voit jamais la clé maître. |

## 2. Chiffrement : ce qui l'est et ce qui ne l'est pas

Le chiffrement est **applicatif** : il est fait dans le core (`src/crypto.rs`)
avant insertion. La base de données ne possède jamais la clé maître
(`ENCRYPTION_KEY`), donc un dump PostgreSQL seul ne permet pas de lire les
secrets.

### Chiffré (AES-256-GCM)

Format stocké : `TEXT` = `base64(nonce || ciphertext || tag)`.

| Table | Colonne | Pourquoi |
|---|---|---|
| `modules` | `hmac_secret` | Secret de signature HMAC : doit pouvoir être **déchiffré** par le core pour vérifier les signatures. |

### Haché de façon déterministe (HMAC-SHA256, irréversible)

| Table | Colonne | Pourquoi |
|---|---|---|
| `modules` | `api_key_hash` | Doit rester **cherchable** (index de recherche à l'authentification) sans jamais stocker la clé API en clair. Déterministe = même clé → même hash. |

### En clair (non chiffré)

| Table(s) | Colonnes | Pourquoi |
|---|---|---|
| `modules` | `id`, `name`, `description`, `is_enabled`, `deleted_at`, `created_at`, `updated_at` | Données non sensibles, nécessaires aux filtres/jointures/affichage admin. |
| `module_scopes` | `module_id`, `scope` | Utilisées pour l'autorisation et les jointures. |
| `hytale_servers` | toutes | `name`, `url` servent à l'affichage et au routage. |
| `player_sessions` | `id`, `hytale_server_id`, `hytale_id`, `joined_at`, `ended_at`, ... | Besoin d'index et de requêtes par `hytale_id` et par date. |
| `whitelists` | `id`, `hytale_server_id`, `hytale_id`, ... | Contrainte d'unicité et recherches par `hytale_id`. |

> Règle : une colonne n'est chiffrée que si elle doit rester **secrète tout en
> étant récupérable**. Si elle doit être **cherchable**, on la hache. Sinon on la
> laisse en clair.

## 3. Schéma cible

### `modules`
Module enregistré auprès du core (website, plugin Hytale, bot Discord...).

| Colonne | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | non | v7 |
| `name` | TEXT | non | UNIQUE (global) |
| `description` | TEXT | **oui** | |
| `api_key_hash` | TEXT | non | HMAC déterministe, UNIQUE, index de recherche |
| `hmac_secret` | TEXT | non | chiffré AES-256-GCM |
| `is_enabled` | BOOLEAN | non | défaut TRUE (remplace `active`) |
| `deleted_at` | TIMESTAMPTZ | oui | |
| `created_at` | TIMESTAMPTZ | non | défaut `now()` |
| `updated_at` | TIMESTAMPTZ | non | défaut `now()` |

> `kind` est **supprimé** au profit de scopes (table `module_scopes`).

### `module_scopes`
Scopes accordés à un module (remplace `kind`).

| Colonne | Type | Null | Note |
|---|---|---|---|
| `module_id` | UUID FK → `modules(id)` | non | `ON DELETE CASCADE` |
| `scope` | TEXT | non | ex. `player_sessions:write`, `whitelist:read` |
| PK | composite `(module_id, scope)` | | |

> L'autorisation par type de service se fait via les scopes. Le webhook vérifie
> que le module appelant possède le scope correspondant à la table métier ciblée.

### `hytale_servers`
Serveur Hytale géré par un module.

| Colonne | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | non | v7 |
| `module_id` | UUID FK → `modules(id)` | non | |
| `name` | TEXT | non | |
| `url` | TEXT | non | |
| `deleted_at` | TIMESTAMPTZ | oui | |
| `created_at` | TIMESTAMPTZ | non | défaut `now()` |
| `updated_at` | TIMESTAMPTZ | non | défaut `now()` |

### `player_sessions`
Session de connexion d'un joueur à un serveur.

| Colonne | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | non | v7 |
| `hytale_server_id` | UUID FK → `hytale_servers(id)` | non | |
| `hytale_id` | UUID | non | identité externe fournie par le plugin/site (pas de table `players`) |
| `joined_at` | TIMESTAMPTZ | non | entrée effective dans le monde |
| `ended_at` | TIMESTAMPTZ | oui | fin de session ; `NULL` = session en cours |
| `deleted_at` | TIMESTAMPTZ | oui | |
| `created_at` | TIMESTAMPTZ | non | défaut `now()` |
| `updated_at` | TIMESTAMPTZ | non | défaut `now()` |

> **Décision : le plugin n'émet une requête qu'une fois le joueur entré dans le
> monde.** On modélise donc uniquement `joined_at` (non nullable) et
> `ended_at`. Pas de `started_at` : aucune source ne fournit l'événement de
> tentative de connexion. Si le suivi des tentatives devient nécessaire, ajouter
> le hook côté plugin **et** la colonne par migration à ce moment-là.

### `whitelists`
Entrée de whitelist d'un joueur pour un serveur.
**Décision : la whitelist est rattachée à un serveur Hytale** (une whitelist
par serveur), afin de gérer plusieurs serveurs indépendamment.

| Colonne | Type | Null | Note |
|---|---|---|---|
| `id` | UUID PK | non | v7 |
| `hytale_server_id` | UUID FK → `hytale_servers(id)` | non | |
| `hytale_id` | UUID | non | |
| `deleted_at` | TIMESTAMPTZ | oui | |
| `created_at` | TIMESTAMPTZ | non | défaut `now()` |
| `updated_at` | TIMESTAMPTZ | non | défaut `now()` |

## 4. Cardinalités (Crow's foot)

```
modules          ||--o{ module_scopes
modules          ||--o{ hytale_servers
hytale_servers   ||--o{ player_sessions
hytale_servers   ||--o{ whitelists
```

- `module_scopes.module_id` : `NOT NULL`.
- `hytale_servers.module_id` : `NOT NULL`.
- `player_sessions.hytale_server_id` : `NOT NULL`.
- `whitelists.hytale_server_id` : `NOT NULL`.

## 5. Index et contraintes

| Table | Contrainte / index | Détail |
|---|---|---|
| `modules` | `UNIQUE (name)` | déjà présent |
| `modules` | `UNIQUE (api_key_hash)` | déjà présent |
| `module_scopes` | PK `(module_id, scope)` | |
| `hytale_servers` | `UNIQUE (module_id, name) WHERE deleted_at IS NULL` | partiel (soft-delete) |
| `hytale_servers` | `INDEX (module_id)` | |
| `player_sessions` | `INDEX (hytale_server_id, joined_at DESC)` | liste des sessions |
| `player_sessions` | `INDEX (hytale_id)` | requête « sessions du joueur » (site) |
| `player_sessions` | `INDEX (ended_at) WHERE ended_at IS NULL` | sessions en cours |
| `whitelists` | `UNIQUE (hytale_server_id, hytale_id) WHERE deleted_at IS NULL` | partiel, permet re-whitelist après suppression |
| `whitelists` | `INDEX (hytale_id)` | vérification « suis-je whitelisté » (site) |

## 6. Suppression de la table `events`

- La table `events` (journal brut) est **supprimée** du modèle cible.
- Le webhook écrit **directement** dans la table métier, restreinte par le scope
  du module authentifié via HMAC (`src/auth/middleware.rs`).

**Compromis assumés :**
- plus de log append-only (audit trail) ni d'idempotence par `event_id` ;
- chaque nouveau type d'événement nécessite une migration ;
- le core devient couplé au domaine Hytale (routage `event_type` → table).

> Si un besoin d'audit/replay réapparaît, prévoir une table `event_log`
> légère (append-only, sans mapping métier) plutôt que de réintroduire la
> logique de projection.

## 7. Décision : pas de table `players`

L'identité joueur (`hytale_id`) reste **dénormalisée** dans `player_sessions`
et `whitelists`.

- `hytale_id` est un UUID fourni par Hytale.
- Côté plugin : passé dans les requêtes pour savoir qui fait quoi.
- Côté website : récupéré via « Se connecter avec Hytale » (OAuth géré dans le
  module website, **pas** dans le core), puis requêtes directes par `hytale_id`.

Raison invoquée : éviter un aller-retour SQL supplémentaire (parse → lookup
player → insert) à chaque connexion.

**Contrepartie acceptée :** si une donnée propre au joueur (display_name,
première/dernière vue, préférences...) doit être persistée un jour, il faudra
réintroduire une table `players` et migrer les `hytale_id` en `player_id`.

## 8. Transport : REST (et non gRPC)

Question posée : remplacer les webhooks REST par gRPC (streaming) pour
l'ingestion des événements.

**Décision : on reste sur une API REST** (webhooks HTTP + endpoints de lecture).

Raisons :

- La charge n'est pas un problème : Axum/Tokio absorbe sans difficulté 20+
  connexions simultanées. Le vrai goulot serait le pool SQLx
  (`max_connections`), pas le transport HTTP.
- Intégration simple côté plugin tiers (un POST HTTP signé est trivial à
  émettre, y compris depuis un plugin de jeu), contre un client gRPC/HTTP-2/TLS
  plus lourd à embarquer.
- Debug et exploitation aisées (`curl`, logs HTTP), contrairement à gRPC qui
  impose `grpcurl` et un load balancer compatible HTTP/2.

**Compromis acceptés :** pas de sémantique de présence temps réel (une
connexion longue), pas de contrat protobuf typé, ack implicite par le 202.

> gRPC reste une option documentée pour plus tard (data-plane typé + stream +
> ack par message), à activer seulement si un vrai besoin de présence/audit
> temps réel apparaît.

## 9. Points encore ouverts

1. **Rétention / purge** RGPD des lignes soft-deleted (volumétrie).

## 10. Impact sur le code existant

| Fichier | Changement |
|---|---|
| `migrations/0001_init.sql` | **Réécrite** en schéma cible (rien n'est commité, pas de `0002`). `events` supprimée, `kind` → `module_scopes`, `active` → `is_enabled`, UUID v7, triggers `updated_at`, index partiels. |
| `src/models.rs` | `ModuleRow` : `kind` → scopes, `active` → `is_enabled`, ajout `deleted_at`. Nouveaux modèles `HytaleServer`, `PlayerSession`, `Whitelist`. |
| `src/db.rs` | Requêtes `events` remplacées par les tables métier ; filtres `deleted_at IS NULL`. |
| `src/routes/webhook.rs` | Routage par scope vers la table métier. |
| `src/auth/middleware.rs` | Vérification du scope après authentification HMAC. |
| `src/routes/events.rs` | Routes `events` à retirer/remplacer. |
| `src/crypto.rs` | Inchangé (AES-256-GCM + HMAC déterministe). |
