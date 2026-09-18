//! Domaine d'orchestration (à venir).
//!
//! Objectif : mettre à jour les serveurs Hytale (récupérer la dernière version
//! publiée, planifier et suivre les mises à jour).
//!
//! Contrainte de conception : les appels réseau sortants ne doivent **jamais**
//! bloquer une requête HTTP. Ils passeront par une table `outbox`/`jobs` et un
//! worker. Le seed et les tests arriveront avec l'implémentation réelle.
