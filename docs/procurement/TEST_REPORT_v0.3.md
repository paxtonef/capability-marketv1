# Rapport d'exécution des tests — PROCUREMENT V0.3

| | |
|---|---|
| Date | 2026-11-20 |
| Branche | `feature/procurement-v0.3` (base : `db30fe2`) |
| Environnement | Linux x86-64, rustc 1.97.0, cargo 1.97.0, Python 3.13 |
| Dépendances | serde 1.0.229, serde_json 1.0.154 |

> **Mise à jour du 2026-11-20 (validation technique).** Le test d'intégration
> CPL B6, non exécuté pendant le développement, l'a été lors de la validation :
> artifact `VALIDATED` sous les versions 1.0.0 et 1.1.0 du contrat, sur une base
> `capability_market_v1_test` locale, avec CPL au commit `3ccdf63`. Le détail est
> dans les livrables de validation (`VALIDATION_REPORT_v0.3.md`,
> `CPL_B6_INTEGRATION_v0.3.md`, `TEST_EXECUTION_v0.3.log`). Les sections
> ci-dessous décrivent l'état à la livraison du développement.

## Synthèse

| Suite | Commande | Résultat |
|---|---|---|
| Tests Rust (debug et release) | `cargo test`, `cargo test --release` | **68 réussis, 0 échec** |
| Analyse statique | `cargo clippy --all-targets`, `cargo fmt --check` | 0 avertissement, format conforme |
| Contrat, sans CPL | `python3 -m unittest tests/test_procurement_v03_contract.py` | **9 réussis, 0 échec** |
| Intégration CPL B6 | `python3 tests/test_procurement_v03_integration.py` | **NON EXÉCUTÉ** (voir ci-dessous) |

## Intégration CPL B6 : non exécutée

Le test d'intégration demande le dépôt CPL (`~/cpl`) et la base
`capability_market_v1_test`. Aucun des deux n'est accessible depuis
l'environnement où ce développement a été réalisé. Le test est livré
(`tests/test_procurement_v03_integration.py`) mais **n'a pas été lancé** ; il doit
l'être sur un poste disposant de CPL avant validation de la Pull Request. La
procédure et la sortie attendue sont dans `INTEGRATION_CPL_B6.md`.

Ce qui a été vérifié à la place, sans CPL :

- les deux fichiers de contrat passent `install_contracts.py` ;
- les charges utiles des deux profils satisfont la règle structurelle déclarée
  par les contrats 1.0.0 et 1.1.0 (champs requis, types racine) ;
- la charge utile 1.1.0 n'a aucun champ racine supplémentaire et conserve chaque
  champ d'élément de la sortie V0.2 avec la même valeur.

Ce qui reste à confirmer par l'exécution réelle : que B6 ne contraint pas le
contenu des tableaux au-delà de leur type.

## Tests Rust

```text
unittests src/lib.rs      43 passed; 0 failed
tests/cli.rs              14 passed; 0 failed
tests/traceability.rs     11 passed; 0 failed
```

### Les cinq tests V0.2

Conservés à l'identique dans `src/legacy.rs` (comparaison textuelle avec
`db30fe2:src/main.rs` : aucune différence) et tous réussis :
`detects_french_requirement`, `detects_english_requirement`,
`reports_no_detected_requirements`, `escapes_json_special_characters`,
`classifies_complex_tender_v02`.

En complément, `profile_1_0_reproduces_the_v02_output_byte_for_byte` établit que
le profil 1.0.0 reproduit `procurement_output_v02.json` octet pour octet.

### Couverture par exigence de l'ordre de développement

| Exigence | Tests |
|---|---|
| Négations | `french_negations_are_prohibitions`, `english_negations_are_prohibitions`, `negated_obligation_is_not_reported_as_obligation`, `negation_keeps_the_whole_marker_as_modality`, `negation_of_necessity_is_an_exemption_not_a_prohibition` |
| Conditions | `leading_condition_makes_a_conditional_obligation`, `trailing_condition_runs_to_the_end_of_the_sentence`, `condition_kinds_are_distinguished`, `conditions_attach_to_other_categories_without_renaming_them`, `unconditional_obligation_has_no_condition`, `every_condition_is_an_exact_slice_inside_its_requirement` |
| Exemptions | `exemptions_in_french_and_english`, `negation_of_necessity_is_an_exemption_not_a_prohibition`, `profile_1_1_is_a_superset_of_profile_1_0` |
| Possibilités facultatives | `optional_provisions_in_french_and_english` |
| Ambiguïtés | `pending_information_is_an_uncertainty_only`, `weak_modality_is_never_promoted_to_an_obligation`, `vague_term_inside_a_requirement_is_flagged`, `vague_term_without_normative_content_is_not_flagged`, `pending_information_inside_a_requirement_is_flagged`, `several_modalities_in_one_sentence_are_flagged`, `negation_without_second_term_is_flagged_not_guessed` |
| Preuve exacte et localisation | `every_evidence_is_the_exact_source_slice_at_its_location`, `every_requirement_text_comes_verbatim_from_its_evidence`, `every_located_uncertainty_is_an_exact_source_slice`, `unusual_input_never_breaks_exact_slicing`, `positions_are_exact_slices_of_the_source` |
| Aucune exigence inventée | `nothing_is_extracted_from_a_document_without_normative_statement`, `descriptive_sentences_carry_no_category`, `section_headings_are_not_turned_into_requirements` |
| Validité du JSON | `v02_invocation_still_works_and_emits_valid_json`, `pretty_and_compact_outputs_carry_the_same_data`, `title_and_identifier_are_escaped_not_interpreted`, `escapes_special_and_control_characters` |
| Déterminisme | `repeated_runs_are_byte_identical` (20 exécutions du binaire × 2 profils × 2 jeux), `output_does_not_depend_on_environment`, `library_runs_are_deterministic` (50 exécutions × 6 sources) |
| Contrat | `both_profiles_satisfy_both_contract_versions`, `identifiers_are_sequential_unique_and_consistent`, `profile_1_1_is_a_superset_of_profile_1_0` |
| CLI | `wrong_usage_exits_with_code_2`, `unreadable_source_exits_with_code_1`, `non_utf8_source_is_refused_rather_than_altered`, `help_and_version_are_available`, `reads_standard_input_and_writes_output_file` |

Les tests de traçabilité portent sur six sources : les trois jeux d'essai, plus
trois variantes du jeu V0.3 (fins de ligne Windows, BOM UTF-8, lignes indentées).

### Sensibilité des tests

Deux altérations volontaires du code ont été appliquées puis retirées, pour
vérifier que la suite les détecte :

| Altération | Résultat |
|---|---|
| `byte_end` des preuves décalé d'un octet | suite en échec |
| Marqueurs d'exemption classés comme obligations | suite en échec |

## Jeux d'essai

| Fichier | Contenu | Sortie de référence |
|---|---|---|
| `runners/procurement/test_tender.txt` | V0.1, 3 obligations | `procurement_output.json` (inchangé) |
| `tests/fixtures/procurement/tender_complex_v1.txt` | V0.2 | `procurement_output_v02.json` (inchangé, profil 1.0.0), `procurement_output_v03.json` (profil 1.1.0) |
| `tests/fixtures/procurement/tender_v03_modalities.txt` | V0.3, français et anglais, 44 lignes | `tests/fixtures/procurement/tender_v03_modalities.expected.json` |

### Résultats sur `tender_complex_v1.txt`

| | V0.2 / profil 1.0.0 | Profil 1.1.0 |
|---|---|---|
| Exigences | 5 | 7 |
| `OBLIGATION` / `PROHIBITION` / `CONDITIONAL_OBLIGATION` | 3 / 1 / 1 | 3 / 1 / 1 |
| `OPTIONAL` / `EXEMPTION` | écartées | 1 / 1 |
| Incertitudes | 1 | 1 (`PENDING_INFORMATION`) |

Les cinq exigences de V0.2 gardent en 1.1.0 leur identifiant, leur catégorie et
leur texte.

### Résultats sur `tender_v03_modalities.txt`

| Catégorie | Nombre |
|---|---|
| `OBLIGATION` | 7 |
| `PROHIBITION` | 6 |
| `CONDITIONAL_OBLIGATION` | 4 |
| `EXEMPTION` | 5 |
| `OPTIONAL` | 4 |
| **Exigences** | **26** |

| Incertitude | Nombre |
|---|---|
| `PENDING_INFORMATION` | 2 |
| `WEAK_MODALITY` | 2 |
| `VAGUE_TERM` | 2 |
| `MULTIPLE_MODALITIES` | 1 |
| `AMBIGUOUS_NEGATION` | 1 |

Les 26 exigences ont été relues une à une contre le texte source lors de la
constitution de la sortie de référence. Les énoncés volontairement non extraits
comme exigences : les six titres de section, « Le prestataire devrait fournir un
rapport mensuel » (modalité faible), « Le marché ne peut être modifié que par
avenant écrit » (négation restrictive, signalée), la phrase à date « 15 May
2026 », et les phrases descriptives.

## Tests Python

```text
test_1_1_keeps_the_root_structure_of_1_0 ... ok
test_both_contracts_are_valid_b6_contracts ... ok
test_schema_root_matches_the_b6_contract ... ok
test_1_1_outputs_add_no_root_field ... ok
test_1_1_outputs_keep_every_1_0_item_field ... ok
test_1_1_outputs_validate_against_the_json_schema ... ok
test_every_reference_output_satisfies_both_contracts ... ok
test_evidence_excerpts_are_exact_source_bytes ... ok
test_runner_reproduces_reference_outputs ... ok

Ran 9 tests — OK
```

Exécuté avec `jsonschema` installé et `PROCUREMENT_BIN` pointant sur le binaire
release. Sans ces deux éléments, les deux tests concernés sont ignorés, non échoués.

## Reproduire

```bash
cd runners/procurement
cargo test && cargo test --release
cargo clippy --all-targets && cargo fmt --check
cargo build --release
cd ../..
PROCUREMENT_BIN=$PWD/runners/procurement/target/release/procurement \
    python3 -m unittest tests/test_procurement_v03_contract.py -v
```
