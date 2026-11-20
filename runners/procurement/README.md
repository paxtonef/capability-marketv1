# PROCUREMENT V0.3 — runner Rust

Extrait les exigences d'un appel d'offres (texte UTF-8, français ou anglais) et
produit un JSON conforme au contrat `cm.procurement.requirements.v1`.

Le runner est autonome : il ne dépend ni de CPL, ni d'une base de données, ni du
réseau. Il n'invente rien : chaque texte émis est une tranche exacte du document
source, accompagnée de sa position.

## Compilation et tests

```bash
cd runners/procurement
cargo build --release          # binaire : target/release/procurement
cargo test                     # tests unitaires et d'intégration
```

Dépendances : `serde` et `serde_json` uniquement. Le binaire n'est pas versionné
(`target/` est ignoré).

## Utilisation

```text
procurement [OPTIONS] TENDER_ID TITLE SOURCE_FILE

  TENDER_ID      Identifiant de l'appel d'offres
  TITLE          Intitulé de l'appel d'offres
  SOURCE_FILE    Fichier texte source, ou - pour l'entrée standard

  --contract-version <V>  Profil de sortie : 1.1.0 (défaut) ou 1.0.0 (sortie V0.2)
  --pretty                JSON indenté
  -o, --output <FILE>     Écrit dans FILE au lieu de la sortie standard
  -h, --help / -V, --version
```

Codes de sortie : `0` succès, `1` erreur de lecture ou d'écriture, `2` erreur d'usage.

```bash
# Profil 1.1.0 (défaut)
procurement CM-COMPLEX-001 "Services linguistiques multilingues" \
    ../../tests/fixtures/procurement/tender_complex_v1.txt

# Sortie identique à V0.2, octet pour octet
procurement --contract-version 1.0.0 CM-COMPLEX-001 "Services linguistiques multilingues" \
    ../../tests/fixtures/procurement/tender_complex_v1.txt

# Depuis l'entrée standard, JSON indenté dans un fichier
pdftotext -layout dce.pdf - | procurement --pretty -o exigences.json AO-2026-17 "Titre" -
```

L'appel historique à trois arguments fonctionne toujours. **Changement par
rapport à V0.2 :** sans option, la sortie est désormais au profil 1.1.0 ; un
consommateur qui a besoin de la forme V0.2 exacte passe `--contract-version 1.0.0`.

## Architecture

| Étape | Module | Rôle |
|---|---|---|
| Lecture | `reader.rs` | Lit le fichier ou l'entrée standard, refuse le non-UTF-8, gère le BOM |
| Segmentation | `segmenter.rs` | Lignes puis phrases, avec ligne, colonne et décalages en octets |
| Classification | `classifier.rs`, `lexicon.rs` | Catégorie, modalité, conditions, signaux d'incertitude d'un énoncé |
| Extraction | `extractor.rs` | Assemble exigences, preuves et incertitudes, attribue les identifiants |
| Sérialisation | `serializer.rs`, `model.rs` | Types du contrat et JSON via `serde` |
| Profil 1.0.0 | `legacy.rs` | Règles V0.2 conservées ; porte les cinq tests d'origine, inchangés |
| CLI | `main.rs` | Analyse des arguments, codes de sortie |

La bibliothèque est utilisable directement :

```rust
use procurement::{ContractVersion, Format, SourceDocument, run};

let document = SourceDocument::from_text("Le prestataire doit livrer sous 5 jours.");
let json = run("AO-1", "Titre", &document, ContractVersion::V1_1, Format::Compact);
```

## Ce qui est extrait

| Catégorie | `binding` | Exemples de marqueurs |
|---|---|---|
| `OBLIGATION` | oui | doit, devra, est tenu de, obligatoire, must, shall, is required |
| `PROHIBITION` | oui | ne doit pas, ne peut en aucun cas, interdit, must not, shall not, may not |
| `CONDITIONAL_OBLIGATION` | oui | une obligation assortie d'au moins une condition |
| `EXEMPTION` | non | n'est pas obligatoire, est dispensé, ne doit pas nécessairement, not required, need not |
| `OPTIONAL` | non | peut, a la possibilité de, facultatif, may, optional |

Conditions (`conditions[].type`) : `IF` (si, en cas de, if), `WHEN` (lorsque, dès
que, when), `PROVIDED_THAT` (à condition que, sous réserve, provided that),
`EXCEPTION` (sauf, à moins que, unless, except). Une condition s'attache à toute
catégorie ; seule l'obligation change de nom (`CONDITIONAL_OBLIGATION`).

Incertitudes (`uncertainties[].kind`) :

| Type | Signification | Donne lieu à une exigence ? |
|---|---|---|
| `PENDING_INFORMATION` | Information annoncée, non fournie (à confirmer, TBD) | seulement si l'énoncé porte aussi une modalité |
| `WEAK_MODALITY` | Modalité non contraignante (devrait, should, recommandé) | jamais à elle seule |
| `VAGUE_TERM` | Terme à marge d'interprétation (si possible, raisonnable, et/ou) | signalé seulement dans un énoncé normatif ou à modalité faible |
| `AMBIGUOUS_NEGATION` | « ne + modal » sans second terme trouvé (« ne peut … que ») | non : le modal n'est pas interprété |
| `MULTIPLE_MODALITIES` | Plusieurs modalités dans une phrase | oui, selon la modalité prioritaire, avec renvoi |
| `NO_REQUIREMENT_DETECTED` | Aucun énoncé normatif dans le document | avis du runner, sans localisation |

Priorité en cas de marqueurs multiples : exemption, interdiction, obligation,
possibilité. Un marqueur recouvert par un marqueur plus prioritaire est ignoré
(« doit » dans « ne doit pas », « required » dans « not required »).

La recherche ignore la casse, les accents et la forme de l'apostrophe, et
respecte les frontières de mot (« peut-être » n'est pas « peut »).

## Garanties vérifiées par les tests

- `evidence[].excerpt` est égal aux octets `byte_start..byte_end` du fichier source.
- `requirements[].text` est égal à l'extrait de sa preuve ; `modality` et le texte
  de chaque condition sont des extraits de cet énoncé.
- Les identifiants `REQ-`, `EV-`, `UNC-` sont uniques, séquentiels dans l'ordre du
  document, et leurs renvois sont cohérents dans les deux sens.
- Deux exécutions sur la même entrée produisent les mêmes octets, quels que
  soient l'environnement, la locale et le répertoire courant.

## Limitations

Le moteur est lexical : il applique des règles sur des marqueurs, sans analyse
syntaxique ni modèle de langue. En conséquence :

1. **Obligations sans marqueur non détectées.** Le présent ou le futur à valeur
   d'obligation (« Le titulaire fournit un rapport », « Le titulaire remettra… »,
   « The supplier will deliver… ») n'est pas reconnu. Ne le sont pas non plus
   « can », « where » et « once ». Seuls le français et l'anglais sont couverts.
2. **Texte UTF-8 uniquement.** Pas de lecture de PDF ou DOCX ; un fichier dans un
   autre encodage est refusé (code 1) plutôt que converti.
3. **Segmentation par ligne physique.** Une phrase coupée sur plusieurs lignes
   (texte issu d'un PDF) est traitée ligne par ligne : la preuve peut être
   partielle et une condition séparée de son obligation. Un texte à un paragraphe
   par ligne donne les meilleurs résultats.
4. **Une exigence par phrase.** Une phrase à plusieurs modalités donne une seule
   exigence, signalée `MULTIPLE_MODALITIES`. Les éléments d'une liste (« doit
   fournir : – un RIB – un Kbis ») ne sont pas rattachés à la phrase introductive.
5. **Conditions.** La clause s'arrête à la première virgule suivie d'un blanc (ou
   à la fin de la phrase) : une condition contenant une virgule est tronquée, tout
   en restant un extrait exact. Une condition énoncée dans une autre phrase n'est
   pas rattachée.
6. **Négations.** Le second terme de la négation est cherché dans les quatre mots
   qui suivent « ne + modal ». Au-delà, ou pour une restriction « ne … que »,
   l'énoncé est signalé `AMBIGUOUS_NEGATION` sans être classé.
7. **Titres de section.** Une ligne numérotée, courte, sans ponctuation finale et
   dont la seule modalité est nominale (« 5. Possibilités facultatives »,
   « 3.2 Assurance obligatoire ») est ignorée. Un titre non numéroté de même
   forme (« Assurance obligatoire ») est, lui, extrait.
8. **Pas de rôle ni de structure.** Le destinataire de l'exigence (candidat,
   titulaire, acheteur) n'est pas extrait. La localisation est donnée en ligne,
   colonne et octets, pas en numéro d'article.
9. **Identifiants positionnels.** `REQ-0003` désigne la troisième exigence du
   document ; l'identifiant change si le document est modifié en amont.
10. **Termes ambigus.** « May » avec majuscule en milieu de phrase est pris pour
    le mois ; « peut » couvre aussi des emplois descriptifs (« le dossier peut
    être téléchargé »), classés `OPTIONAL`.
11. **Profil 1.0.0.** Les règles sont celles de V0.2, y compris leurs limites. Le
    JSON est équivalent ; seul l'échappement de quelques caractères de contrôle
    rares peut différer dans sa forme (`\b` au lieu de `\u0008`).

Le runner signale ce qu'il détecte ; il ne garantit pas l'exhaustivité. Sa sortie
est une aide à la lecture d'un appel d'offres, pas un substitut.
