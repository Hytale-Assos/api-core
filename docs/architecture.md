# Architecture — api-core

Dernière mise à jour : 18 septembre 2026

`api-core` est le **hub** qui met en relation tous les services de l'écosystème
Hytale. Les consommateurs ne se parlent jamais directement : ils passent tous
par le core.

```
plugin Hytale ─┐
site Laravel   ─┼──►  api-core (hub)  ──►  PostgreSQL
bot Discord    ─┘         │
                          └──► orchestration (futur) ──► serveurs Hytale
```

## Règles fondatrices

1. **Un consommateur = un module.** Le plugin, le site Laravel, le bot Discord,
   le backoffice et (plus tard) l'orchestration sont tous enregistrés comme des
   modules authentifiés par API key + HMAC. Il n'y a **qu'une seule logique
   d'authentification** : pas d'endpoints admin protégés par un token séparé.
2. **Le core ne connaît que des modules et des `hytale_id`.** Jamais de compte
   utilisateur. La création de compte, l'OAuth et le mapping compte ↔ `hytale_id`
   vivent dans le site Laravel (et le mapping user Discord ↔ `hytale_id` dans le
   bot), pas dans le core.
3. **`hytale_id` est un filtre de données, pas une identité.** L'autorisation
   repose uniquement sur le `Principal` authentifié (module + scopes).
4. **Ownership vs visibilité.** L'écriture sur un serveur est restreinte à son
   module propriétaire (`hytale_servers.module_id`). La lecture est globale pour
   tout module disposant du scope `:read` correspondant : consulter n'est pas
   risqué, seules les modifications sont restreintes. Le scope `admin` contourne
   le contrôle d'ownership (backoffice).
5. **Aucun appel réseau sortant dans le chemin d'une requête HTTP.** Les effets
   longs (orchestration) passeront par une table `outbox`/`jobs` et un worker.

## Structure du code

```
src/
├── main.rs                # bootstrap : config, pool, migrations, seed, serve
├── config.rs              # env -> Config
├── error.rs               # AppError + IntoResponse
├── state.rs               # AppState { db, crypto, config }
├── crypto.rs              # AES-256-GCM + HMAC déterministe
├── testutil.rs            # helpers de test (#[cfg(test)])
├── http/                  # transport uniquement
│   ├── mod.rs
│   ├── router.rs          # assemblage des routes + layers
│   ├── middleware.rs      # module_auth (API key + HMAC)
│   ├── extractors.rs      # Principal, ...
│   └── health.rs          # GET /health
├── shared/                # utilitaires transverses
│   ├── mod.rs
│   ├── hmac.rs            # vérification de signature de requête
│   └── pagination.rs      # Page<T>, pagination
├── domains/
│   ├── modules/           # registre + gestion des modules
│   ├── servers/           # serveurs Hytale
│   ├── sessions/          # sessions joueur
│   ├── whitelists/        # whitelist joueur
│   └── orchestration/     # (futur) mise à jour des serveurs
└── ingest/
    ├── mod.rs
    └── webhook.rs         # WebhookEvent -> sessions::service
```

Chaque domaine suit toujours le même découpage (fichiers plats) :

| Fichier | Rôle |
|---|---|
| `model.rs` | rows `FromRow`, vues `Serialize`, requêtes `Deserialize` |
| `repo.rs` | SQL pur, aucune règle métier |
| `service.rs` | règles métier, ownership, idempotence, validations |
| `handler.rs` | extraction HTTP, vérification de scope, appel `service` |

## Frontières

- `service` ne dépend **jamais** de `http`. Il est testable sans serveur.
- `handler` ne contient **aucun SQL** et reste mince (~10 lignes).
- `repo` ne contient **aucune règle** (pas d'`assert_*`).
- `http` est le seul endroit qui parle Axum.
- Les traits de repository ne sont pas introduits tant qu'il n'y a qu'un backend
  (`#[sqlx::test]` couvre la testabilité DB).

## Flux d'une requête

```
Laravel / bot / plugin
        │  X-Api-Key (+ HMAC si écriture)
        ▼
http::middleware module_auth ──► Principal (module + scopes)
        ▼
domains::<domaine>::handler
        │  require(scope), extrait hytale_id / filtres
        ▼
domains::<domaine>::service ──► autre service (ownership)
        ▼
domains::<domaine>::repo ──► PostgreSQL
```

## Amorçage (seed)

Créer un module exige d'être un module : il faut un point de départ. Au
démarrage, si `SEED_ADMIN_API_KEY` et `SEED_ADMIN_HMAC_SECRET` sont définis et
qu'aucun module nommé `admin` n'existe, le core le crée avec tous les scopes.
Le seed est idempotent. Pour gérer ensuite les autres modules : `curl`.
