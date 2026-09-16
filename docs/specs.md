# Specs — API Rust (Axum)

Dernière vérification : 15 septembre 2026

## 1. Objectif

API HTTP en Rust capable de :
- gérer plusieurs requêtes simultanément (multi-thread / async) ;
- exposer un endpoint webhook (réception d'événements POST) ;
- se connecter à une base de données relationnelle.

## 2. Stack retenue et état de maintenance

| Crate | Rôle | Dernière version (sept. 2026) | Statut |
|---|---|---|---|
| `axum` | Framework web | **0.8.7** | ✅ Activement maintenu par l'équipe Tokio, releases régulières |
| `tokio` | Runtime async / multi-thread | **1.53.1** | ✅ Très activement maintenu. Deux branches LTS en cours : 1.47.x (support jusqu'à sept. 2026) et 1.51.x (support jusqu'à mars 2027) |
| `sqlx` | Accès BDD async | **0.9.0** (0.8.6 encore très répandu) | ✅ Maintenu, mais release process partiellement manuel — bien vérifier les changelogs lors d'un upgrade majeur |
| `tower` / `tower-http` | Middlewares (logs, CORS, timeouts...) | `tower` 0.5.3 / `tower-http` 0.7.1 | ✅ Maintenu, intégré nativement à l'écosystème Axum |
| `serde` / `serde_json` | Sérialisation JSON | **1.0.229** | ✅ Extrêmement stable, maintenu depuis des années, quasi-standard de facto |
| `axum-extra` (optionnel) | Utilitaires Axum (cookies, typed headers...) | 0.12.6 | ✅ Maintenu en parallèle d'Axum |

**Verdict global : toute la stack proposée est activement maintenue en septembre 2026.** Aucun signal d'abandon détecté. Point de vigilance : `sqlx` publie parfois des versions majeures (0.8 → 0.9) donc figer la version dans `Cargo.toml` et lire le changelog avant de monter de version.

## 3. Choix du framework : Axum vs Actix-web

| Critère | Axum | Actix-web |
|---|---|---|
| Modèle de concurrence | Un seul runtime Tokio multi-thread, work-stealing | Acteurs / plusieurs runtimes Tokio mono-thread |
| Perf brute (req/s saturé) | ~18 000 | ~20 000 |
| RAM idle | 60–90 MB | 50–80 MB |
| Écosystème | Tower/Hyper/SQLx — cohérent avec l'async Rust "standard" | Plus autonome, historique |
| Maintenabilité / courbe d'apprentissage | Plus simple, handlers = fonctions async classiques | Plus complexe (modèle acteur), mais très performant |

**Décision : Axum.** L'écart de perf (~10-15%) n'a pas d'impact réel pour la majorité des API (le goulot d'étranglement est presque toujours la BDD ou la logique métier, pas le framework). Axum offre une meilleure ergonomie et une intégration plus naturelle avec SQLx et Tower.

## 4. Multi-threading / concurrence

- Pas de gestion manuelle de threads nécessaire.
- Le runtime Tokio (`#[tokio::main]`, flavor `multi_thread` par défaut) répartit chaque requête entrante comme une tâche asynchrone légère sur un pool de threads OS, avec un ordonnanceur *work-stealing*.
- Chaque `async fn` handler dans Axum peut être exécuté concurremment sans blocage — tant qu'on évite le code bloquant synchrone (sinon utiliser `tokio::task::spawn_blocking`).

```rust
#[tokio::main] // multi-thread par défaut
async fn main() {
    // ...
}
```

## 5. Connexion base de données

- Driver retenu : **SQLx** (async natif, compatible Postgres / MySQL / SQLite, vérification des requêtes à la compilation).
- Alternative si besoin d'un vrai ORM typé : **SeaORM** (construit sur SQLx).
- Gestion du pool de connexions via `sqlx::Pool`, partagé entre les handlers via l'état Axum (`State`).

```rust
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
```

## 6. Endpoint Webhook

- Un webhook = une route HTTP POST classique exposée par l'API.
- Bonnes pratiques :
  - vérifier la signature de la requête (header HMAC fourni par l'émetteur du webhook) ;
  - répondre rapidement (200 OK) puis traiter l'événement de façon asynchrone si le traitement est long, pour éviter les timeouts / retries côté émetteur ;
  - journaliser les payloads reçus pour debug.

```rust
async fn webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    // 1. vérifier la signature (header HMAC)
    // 2. traiter / enregistrer l'événement
    StatusCode::OK
}
```

## 7. Prochaines étapes suggérées

1. Générer le squelette de projet (`Cargo.toml` + `main.rs` + structure de dossiers).
2. Définir le schéma de base de données et les migrations (`sqlx-cli`).
3. Implémenter les routes principales + le endpoint webhook.
4. Ajouter les middlewares Tower (logs, CORS, rate limiting) selon besoin.
5. Tests d'intégration + config de déploiement.

## 8. Sources consultées

- Comparatifs Axum vs Actix-web 2026 (Medium, Rustify)
- crates.io / docs.rs pour les numéros de version (axum, tokio, sqlx, serde, tower)
