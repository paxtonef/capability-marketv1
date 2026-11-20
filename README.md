# Capability Market V1 — État du dépôt et reprise à froid

**État de référence :** 20 novembre 2026  
**Dépôt :** `paxtonef/capability-marketv1` — branche `main`  
**Dernier commit de consolidation documentaire connu :** `06f7f1d`  
**Statut :** socle PROCUREMENT V0.3 disponible ; Capability Market V1 dans son ensemble n'est pas livré.

## 1. Objet

Capability Market V1 vise à rapprocher les exigences des appels d'offres des capacités mobilisables par des entreprises et des professionnels. L'architecture fonctionnelle envisagée relie l'extraction des exigences (PROCUREMENT), la caractérisation des capacités (CBS), la résolution des écarts de capacités et la découverte de partenaires. Ces éléments constituent une **cible**, et non une liste de modules déjà implémentés.

Ce dépôt contient aujourd'hui principalement le runner **PROCUREMENT V0.3**, ses contrats, ses jeux d'essai et ses preuves de validation.

## 2. Périmètre effectivement présent

| Élément | Statut dans ce dépôt |
|---|---|
| Runner Rust PROCUREMENT V0.3 | Implémenté ; CLI et bibliothèque |
| Extraction de modalités FR/EN à partir de texte UTF-8 | Implémentée, à base de règles lexicales |
| Traçabilité par extraits et positions en octets | Implémentée et testée |
| Sortie JSON `cm.procurement.requirements.v1` | Profils 1.0.0 et 1.1.0 |
| Contrats, fixtures, sorties de référence | Présents |
| Tests Rust et Python | Présents ; résultats historiques documentés |
| Enregistrement d'artefacts dans CPL B6 | Script/test d'intégration présent ; CPL externe requis |
| CBS, résolution des écarts, découverte de partenaires | Aucune implémentation identifiée dans ce dépôt |
| Application ou interface Capability Market complète | Non présente dans ce dépôt |

**Ne pas assimiler la validation du runner à celle du produit Capability Market V1.**

## 3. Parcours de lecture

1. `runners/procurement/README.md` — fonctionnement, CLI, architecture et limites.
2. `docs/procurement/CONTRACT_v1_1.md` — évolution du contrat de sortie.
3. `docs/procurement/INTEGRATION_CPL_B6.md` — enregistrement, prérequis et test d'intégration CPL.
4. `docs/procurement/TEST_REPORT_v0.3.md` — tests de développement et mise à jour de validation.
5. `docs/procurement/validation/` — rapports et journal de vérification indépendante.

## 4. Prérequis et tests

Pour le runner autonome : chaîne d'outils Rust/Cargo compatible avec `runners/procurement/Cargo.lock`, et Python 3 pour les tests de contrat. Les dépendances Rust déclarées sont `serde` et `serde_json`. Certains contrôles Python utilisent `jsonschema` lorsqu'il est installé ; l'absence d'outils optionnels peut entraîner des tests ignorés. Vérifier les prérequis effectifs avant de déclarer une campagne complète.

Depuis la racine du dépôt :

```bash
cd runners/procurement
cargo build --release
cargo test
cargo test --release
cargo clippy --all-targets
cargo fmt --check
cd ../..
PROCUREMENT_BIN="$PWD/runners/procurement/target/release/procurement" python3 -m unittest tests/test_procurement_v03_contract.py -v
```

Le runner accepte un fichier texte UTF-8 ou l'entrée standard et produit un JSON. Voir sa documentation pour la syntaxe exacte, les codes de sortie et les profils de contrat.

### Intégration CPL B6 (facultative pour compiler PROCUREMENT)

Le dépôt **CPL est externe**. L'intégration requiert un environnement CPL compatible, PostgreSQL, le registre des schémas B6 et une base de test nommée `capability_market_v1_test`. Consulter impérativement `docs/procurement/INTEGRATION_CPL_B6.md` avant toute exécution. Les tests doivent rester confinés à la base de test et se terminer par rollback.

La validation indépendante historique a utilisé CPL au commit `3ccdf63a8e2fd2879bbb9eea71053ab11c176e7c`. **Attention :** le répertoire de travail CPL local signalé lors de la consolidation contient des modifications non commitées ; son HEAD seul ne garantit pas l'identité des fichiers exécutés. Ne pas réinitialiser ni modifier ce dépôt pour reproduire les tests. Préférer une copie ou un worktree isolé au commit de référence.

Pour conclure à la validation B6, vérifier **`artifact_status == VALIDATED` et `structurally_valid is True`** : le retour `SUCCESS` de `register_artifact` ne suffit pas. La validation structurelle B6 porte sur les champs racine et leurs types ; elle ne vérifie pas intégralement le contenu des tableaux.

## 5. Résultats historiques documentés

Le rapport de tests du 20 novembre 2026 indique **68 tests Rust réussis**, **9 tests Python réussis**, ainsi que Clippy et le contrôle de formatage conformes. Le test CPL n'avait pas été exécuté **pendant le développement** ; la validation technique indépendante ultérieure a confirmé l'intégration B6 pour les deux versions de contrat (1.0.0 et 1.1.0). Les preuves correspondantes se trouvent dans `docs/procurement/validation/`.

Ces résultats sont des **preuves historiques** : un nouvel intervenant doit les reproduire dans son propre environnement et consigner les versions, commandes, résultats et éventuels écarts.

## 6. Limites connues et risques de reprise

- Moteur lexical, sans compréhension syntaxique ou sémantique complète ; certaines obligations implicites ou sans marqueur ne sont pas détectées.
- Entrée UTF-8 uniquement ; pas de prise en charge native des PDF/DOCX.
- Segmentation par ligne physique et une exigence par phrase : attention aux documents convertis depuis des PDF, aux listes et aux phrases multilignes.
- Couverture limitée au français et à l'anglais ; faux positifs et faux négatifs possibles.
- Identifiants d'exigences positionnels, susceptibles de changer si le texte source est modifié.
- Les tests et contrats prouvent la conformité de scénarios connus, **pas l'exhaustivité sur des appels d'offres réels**.
- Les autres composants fonctionnels de Capability Market V1 restent à spécifier et/ou implémenter.

Pour les limitations détaillées, se référer à `runners/procurement/README.md` plutôt qu'à cette synthèse.

## 7. Procédure de reprise à froid

1. Cloner le dépôt et relever le commit exact, la plateforme et les versions des outils.
2. Lire les cinq ensembles documentaires listés en section 3 ; distinguer preuves historiques et nouveaux résultats.
3. Compiler le runner et reproduire les tests Rust et Python sans CPL.
4. Comparer les sorties aux fixtures de référence et vérifier les invariants de traçabilité.
5. Si l'intégration est dans le périmètre de la reprise, utiliser un CPL **isolé** au commit de référence, une base PostgreSQL de test et les procédures B6 documentées.
6. Consigner les résultats, les dépendances, les écarts, les limites et les éventuels blocages dans un rapport de vérification indépendant.
7. Ne pas déclarer Capability Market V1 fonctionnel au-delà du périmètre réellement testé.

## 8. Gouvernance de livraison

**DEV → VÉRIFICATION indépendante → DEVOPS → `install.sh` → INSTALLATION**

- **DEV :** livre code, contrats, documentation, fixtures et preuves de tests.
- **VÉRIFICATION indépendante :** reproduit les résultats, contrôle les limites et statue sur les écarts.
- **DEVOPS :** intervient après acceptation explicite du dossier vérifié ; prépare la distribution et l'automatisation de l'installation.
- **INSTALLATION :** n'est engagée qu'après validation de la procédure d'installation.

**Aucun `install.sh` global n'est actuellement identifié dans ce dépôt.** Ce document n'en promet pas l'existence. Le présent README ne constitue ni une approbation DEVOPS ni une recette fonctionnelle de Capability Market V1.

## 9. Travaux encore ouverts

- Préciser les contrats d'interface entre PROCUREMENT et les futurs composants CBS / résolution des écarts / découverte de partenaires.
- Définir des critères d'acceptation fonctionnelle sur des appels d'offres réels et représentatifs.
- Consolider les prérequis de l'environnement de vérification et la stratégie d'isolation de CPL.
- Ajouter, si décidé après vérification, une automatisation CI et une procédure d'installation reproductible.

**Règle de conservation :** ne pas réécrire les preuves historiques pour leur faire refléter des résultats postérieurs ; documenter séparément toute nouvelle vérification.
