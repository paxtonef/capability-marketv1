# Rapport de validation technique — PROCUREMENT V0.3

| | |
|---|---|
| Date | 2026-11-20 |
| Dépôt | https://github.com/paxtonef/capability-marketv1 |
| Branche | `feature/procurement-v0.3` (base `db30fe2`) |
| **Commit testé** | **`d8d675e71441d66fa007995f4ded6b6e11e22d2a`** (arbre `6747e1cfb91bfc192dd42d4a9d91694462362f10`) |
| Pull Request | **non ouverte** — voir anomalie A1 |
| CPL utilisé | `paxtonef/cpl` @ `3ccdf63a8e2fd2879bbb9eea71053ab11c176e7c`, lecture seule |
| PI consulté | `paxtonef/cpl-product-integration` @ `dc3ac3283cc3c5cb274171b11fd39a613c370189`, lecture seule |
| Journal | `TEST_EXECUTION_v0.3.log` (toutes les sorties citées ici en proviennent) |

## Verdict

Les contrôles techniques sont tous **PASS**. Deux réserves empêchent de déclarer
la livraison prête à fusionner :

1. **La branche n'est pas sur GitHub et la Pull Request n'est pas ouverte.**
   L'accès en écriture au dépôt m'est refusé (HTTP 403). Le commit testé est
   livré sous forme de bundle Git, qui en conserve le hash.
2. **L'intégration a été exécutée sur une instance PostgreSQL montée pour cette
   validation**, pas sur votre base `capability_market_v1_test` existante. Le
   code de CPL est le vrai (`3ccdf63`), le schéma vient de ses migrations, la
   base porte le nom imposé. L'état de votre propre base (niveau de migration,
   contenu du registre B6) n'a pas pu être contrôlé.

## Environnement

| Outil | Version |
|---|---|
| rustc / cargo | 1.97.0 / 1.97.0 |
| clippy / rustfmt | 0.1.97 / 1.9.0-stable |
| serde / serde_json | 1.0.229 / 1.0.154 |
| Python | 3.13.18 |
| SQLAlchemy / alembic / psycopg2-binary | 2.1.9 / 1.20.1 / 2.9.13 |
| pydantic / pydantic-settings / structlog | 2.14.4 / 2.16.3 / 26.1.0 |
| jsonschema | 4.26.0 |
| PostgreSQL | 16.15 |
| Système | Linux 6.18 x86-64 |

Instance PostgreSQL : locale, écoute sur 127.0.0.1 uniquement, créée vide. Elle
ne contient que `capability_market_v1_test` et la base système `postgres`. Les
bases `cpl_db`, `pi01_test` et `test` n'y existent pas. Schéma : migrations CPL
001 à 028 appliquées sans modification.

Les tests ont porté sur un clone propre du commit `d8d675e`, pas sur le
répertoire de développement.

## Contrôles

### §2 — Environnement de validation

| # | Contrôle | Résultat | Preuve (journal) |
|---|---|---|---|
| 2.1 | Dépôt CPL et PostgreSQL disponibles | PASS, avec réserve 2 | §0, §1 |
| 2.2 | Seule base utilisée : `capability_market_v1_test` | PASS | §1 : `current_database()` ; chaque test l'affiche |
| 2.3 | Connexion vérifiée explicitement avant exécution | PASS | garde sur l'URL puis `current_database()` dans la session |
| 2.4 | `cpl_db`, `pi01_test`, `test` jamais utilisées | PASS | §3.1 : 8 refus sur 8 avant toute connexion ; bases absentes de l'instance |
| 2.5 | Code de CPL non modifié | PASS | §6 : `git status` vide, HEAD `3ccdf63` |
| 2.6 | Code de PI non modifié | PASS | §6 : `git status` vide, HEAD `dc3ac32` |

### §3 — Code livré

| # | Contrôle | Résultat | Preuve |
|---|---|---|---|
| 3.1 | Construction du runner (`cargo build --release`) | PASS | §2 |
| 3.2a | Tests Rust, debug | PASS — 68 réussis, 0 échec, 0 ignoré | §2 : 43 + 14 + 11 |
| 3.2b | Tests Rust, release | PASS — 68 réussis, 0 échec, 0 ignoré | §2 : 43 + 14 + 11 |
| 3.3a | Clippy (`--all-targets -- -D warnings`) | PASS — 0 avertissement | §2 |
| 3.3b | Formatage (`cargo fmt --check`) | PASS | §2 |
| 3.4 | Tests contractuels Python, `jsonschema` installé, `PROCUREMENT_BIN` défini | PASS — 9 réussis, 0 échec | §2 |
| 3.5 | Aucun test ignoré silencieusement | PASS | Rust : `0 ignored` partout, `--ignored` n'en trouve aucun ; Python : 9 `ok`, aucun `skipped` |

### §4 — Intégration réelle CPL B6

| # | Contrôle | Résultat | Preuve |
|---|---|---|---|
| 4.1 | Connexion à `capability_market_v1_test` | PASS | `DATABASE: capability_market_v1_test` |
| 4.2 | Admission de l'exécution du runner | PASS | `EXECUTION: SUCCESS` ; SQL : `runner_type=PROCUREMENT`, `runner_version=0.3.0` |
| 4.3 | Enregistrement de l'artifact PROCUREMENT | PASS | `ARTIFACT: SUCCESS` ; SQL : `artifact_type=TENDER_REQUIREMENTS` |
| 4.4 | Validation sous `cm.procurement.requirements.v1` 1.0.0 | PASS | §3.5 |
| 4.5 | Enregistrement du contrat 1.1.0 au registre B6 de la base de test | PASS | §3.7 : `REGISTERED`, puis `ALREADY REGISTERED` au second appel |
| 4.6 | Validation de l'artifact sous 1.1.0 | PASS | §3.8 |
| 4.7 | Statut `VALIDATED` et `structurally_valid = true`, contrôlés explicitement | PASS, sous les deux versions | relecture SQL directe de `cpl.runner_artifacts` |
| 4.8 | Rollback des données de test | PASS | §4 : toutes les tables à 0, hors registre |

Deux témoins négatifs confirment que le contrôle du statut est effectif : avec un
contrat absent du registre, l'appel renvoie `SUCCESS` mais l'artifact est
`REJECTED`, et le test échoue (§3.2 et §3.6). Détail : `CPL_B6_INTEGRATION_v0.3.md`.

### §5 — Compatibilité fonctionnelle

| # | Contrôle | Résultat | Preuve |
|---|---|---|---|
| 5.1 | Le profil 1.0.0 reproduit la sortie V0.2 attendue | PASS | identique octet pour octet à `procurement_output_v02.json` ; les 3 tests d'intégration V0.1/V0.2, inchangés, passent sur CPL |
| 5.2 | Le profil 1.1.0 conserve les informations existantes | PASS, avec précision | voir ci-dessous |
| 5.3 | `OPTIONAL` et `EXEMPTION` non interprétés comme obligations | PASS | catégories distinctes, toujours `binding: false` ; 2 + 9 éléments contrôlés |
| 5.4 | `binding: false` distingué des exigences contraignantes | PASS | `binding` vrai si et seulement si la catégorie est `OBLIGATION`, `PROHIBITION` ou `CONDITIONAL_OBLIGATION` ; lisible aussi en SQL (`binding_requirements: 5` sur 7) |
| 5.5 | Chaque preuve reliée à son passage exact | PASS | 7 + 26 preuves : `excerpt` = octets `byte_start..byte_end` du fichier, à la ligne indiquée ; lien preuve ↔ exigence vérifié dans les deux sens |
| 5.6 | Aucun changement nécessaire dans CPL ou PI | PASS | artifacts `VALIDATED` avec CPL `3ccdf63` intact ; le contrat 1.1.0 est une ligne de données au registre |
| 5.7 | Consommateurs ignorant `binding` | Aucun dans CPL ni PI ; voir A4 | recherche dans les trois dépôts |

**Précision sur 5.2.** Sur le jeu de référence V0.2, les cinq exigences se
retrouvent en 1.1.0 avec le même identifiant, la même catégorie et le même
texte, et chaque champ d'élément garde sa valeur. Ce n'est pas une identité
générale : les deux profils appliquent des règles différentes. Sur le jeu V0.3,
les 14 lignes retenues par le profil 1.0.0 sont toutes couvertes en 1.1.0, mais
8 y sont classées autrement — ce sont des erreurs de classement de V0.2 que V0.3
corrige :

| Ligne | Profil 1.0.0 (règles V0.2) | Profil 1.1.0 |
|---|---|---|
| 5 | une `OBLIGATION` pour deux phrases | deux `OBLIGATION` |
| 8 | `OBLIGATION` (« … obligatoire ; … facultative ») | `OBLIGATION` + `OPTIONAL` |
| 12 | `OBLIGATION` (« ne doit en aucun cas ») | `PROHIBITION` |
| 19, 20, 21 | `OBLIGATION` (condition non reconnue) | `CONDITIONAL_OBLIGATION` |
| 27 | `PROHIBITION` (« ne doivent pas nécessairement ») | `EXEMPTION` |
| 30 | `PROHIBITION` (« shall not be required ») | `EXEMPTION` |

### §7 — Conditions de réception

| Condition | État |
|---|---|
| Tous les tests obligatoires réussissent | Remplie |
| Les deux versions du contrat sont validées par CPL B6 | Remplie sur l'instance de validation (réserve 2) |
| Aucune régression V0.2 | Remplie |
| Les données de test ne persistent pas après rollback | Remplie |
| Aucun dépôt ou environnement de production modifié | Remplie — rien n'a été poussé, CPL et PI intacts, aucune base de production accessible ni contactée |
| Lien de la Pull Request | **NON EXÉCUTÉ** (A1) |

Rien n'a été fusionné ni déployé.

## Anomalies et corrections proposées

| # | Gravité | Constat | Correction proposée |
|---|---|---|---|
| A1 | Bloquante pour la réception | Push refusé (HTTP 403) : la branche n'existe pas sur GitHub, la PR n'est pas ouverte. | Installer l'application GitHub de Claude sur le dépôt, ou importer `procurement-v0.3.bundle` (commandes ci-dessous) et ouvrir la PR avec `PULL_REQUEST.md`. |
| A2 | Réserve | Intégration exécutée sur une instance locale neuve, pas sur votre base de test. | Sur votre poste : enregistrer le contrat 1.1.0 avec `tests/register_contract_b6.py`, puis lancer le test sous 1.0.0 et 1.1.0. Une dizaine de secondes. |
| A3 | Information | Un contrat absent du registre B6 donne un appel `SUCCESS` et un artifact `REJECTED`. Comportement de CPL, antérieur à V0.3. | Aucune dans CPL. Côté appelant : toujours contrôler `artifact_status`. Documenté dans `INTEGRATION_CPL_B6.md`. |
| A4 | Faible | `tests/test_procurement_v02_integration.py` et `tests/test_procurement_rust_integration.py` comptent `len(payload["requirements"])` sans regarder `binding`. Ils lisent des fichiers figés au profil 1.0.0 et passent. Ils échoueraient si ces fichiers étaient régénérés avec le profil par défaut (7 éléments au lieu de 5). | Ne régénérer ces fichiers qu'avec `--contract-version 1.0.0`. Tout nouveau consommateur doit filtrer sur `binding`. |
| A5 | Faible | Avec SQLAlchemy 2.1, une URL `postgresql://` appelle le pilote `psycopg` v3, que CPL ne déclare pas (`psycopg2-binary`). | Utiliser `postgresql+psycopg2://`, ou fixer `sqlalchemy<2.1` dans l'environnement CPL. Aucun changement de code. |
| A6 | Information | Les trois tests d'intégration V0.1/V0.2 lisent `~/Projects/capability-market-v1/…` en dur. Antérieur à V0.3 ; contourné ici par un lien symbolique. | Hors périmètre. Le test V0.3 utilise un chemin relatif au dépôt. |

Par rapport au patch livré à l'issue du développement (`427ebf3`), le commit
testé ajoute `tests/register_contract_b6.py` et met à jour deux documents. Le
runner n'a pas été modifié.

## Importer le commit testé

```bash
cd capability-marketv1            # clone à jour, main = db30fe2
git fetch /chemin/vers/procurement-v0.3.bundle feature/procurement-v0.3:feature/procurement-v0.3
git rev-parse feature/procurement-v0.3     # doit afficher d8d675e71441d66fa007995f4ded6b6e11e22d2a
git push -u origin feature/procurement-v0.3
```

Le fichier `procurement-v0.3.patch` (`git am`) donne le même contenu — arbre
`6747e1cf…` — mais des hash de commit différents.
