# AML Transaction Monitoring System

Rust · Axum 0.8 · SeaORM 1.1 · PostgreSQL 16. Scores every payment in real time against a
configurable typology rule set, raises de-duplicated alerts, and carries them through an
analyst → case → SAR workflow with an immutable audit trail.

```
crates/
  aml_datasets   adapters for SAML-D, AML_Suspicious, FraudShield, Risk_prediction -> canonical transactions
  aml_engine     streaming detection engine (no DB, no async) — rules, scoring, rolling windows
  aml_eval       `aml-eval`: back-test the engine on the labelled SAML-D dataset
  aml_entity     SeaORM entities
  aml_migration  SeaORM migration (schema, constraints, indexes, append-only audit trigger)
  aml_api        `aml` binary: Axum API + CLI (migrate | serve | import-saml)
```

## Quick start

```powershell
& "C:\Program Files\PostgreSQL\18\bin\psql.exe" -U postgres
Option 1: create the role and database (PowerShell)

Use the postgres superuser password you set when you installed PostgreSQL:

powershell
psql -U postgres -h localhost -c "CREATE USER aml WITH PASSWORD 'aml_dev_pw';"
psql -U postgres -h localhost -c "CREATE DATABASE aml OWNER aml;"

Type these at the postgres=# prompt, one at a time, pressing Enter after each:
postgres=# CREATE USER aml WITH PASSWORD 'aml_dev_pw';

You should see CREATE DATABASE. If it says the database already exists,
 run ALTER DATABASE aml OWNER TO aml;.

ALTER USER aml WITH PASSWORD 'aml_dev_pw';
postgres=# CREATE DATABASE aml OWNER aml;

powershell
cd C:\Users\cyber-Willz\Desktop\aml_system
$env:DATABASE_URL = "postgres://aml:aml_dev_pw@localhost:5432/aml"
$env:AML_API_KEYS = "adminkey-0123456789:admin:mlro,analystkey-0123456789:analyst:bob,ingestkey-0123456789:ingest:core"
cargo run --release -p aml_api -- serve

PS C:\Users\cyber-Willz\Desktop\aml_system> Invoke-RestMethod http://127.0.0.1:8080/health

database     engine_accounts     status
--------     ---------------     ------
    True               0           ok    

PS C:\Users\cyber-Willz\Desktop\aml_system> $h = @{ "x-api-key" = "ingestkey-0123456789" }
>> $tx = @{ sender_account = 6100001; receiver_account = 6100002; amount = 250; sender_country = "GB"; receiver_country = "RU"; payment_type = "WIRE" } | ConvertTo-Json
>> Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8080/api/v1/transactions -Headers $h -ContentType "application/json" -Body $tx | ConvertTo-Json -Depth 5
{
    "transaction_id":  1,
    "risk_score":  75,
    "severity":  "high",
    "flagged":  true,
    "hits":  [
                 {
                     "rule":  "SANCTIONED_JURISDICTION",
                     "weight":  0.75,
                     "side":  "receiver",
                     "detail":  "sanctioned jurisdiction: GB -\u003e RU"
                 }
             ],
    "linked_transaction_ids":  [

                               ],
    "alert_id":  1
}

Try the analyst workflow on that alert

$a = @{ "x-api-key" = "analystkey-0123456789" }
$j = "application/json"

# 1. open the alert (shows the explanation and the related transactions)
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/v1/alerts/1 -Headers $a | ConvertTo-Json -Depth 6

# 2. take it into review and assign it
Invoke-RestMethod -Method Patch -Uri http://127.0.0.1:8080/api/v1/alerts/1 -Headers $a -ContentType $j `
  -Body (@{ status = "under_review"; assigned_to = "bob" } | ConvertTo-Json)

# 3. open a case and attach the alert
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8080/api/v1/cases -Headers $a -ContentType $j `
  -Body (@{ title = "Payment to sanctioned jurisdiction"; priority = "high"; alert_ids = @(1) } | ConvertTo-Json)

{
    "alert":  {
                  "assigned_to":  null,
                  "case_id":  null,
                  "created_at":  "2026-10-25T17:58:09.016036Z",
                  "dedupe_key":  "6100002:20261025:SANCTIONED_JURISDICTION",
                  "explanation":  {
                                      "amount_gbp":  250.0,
                                      "hits":  [
                                                   {
                                                       "detail":  "sanctioned jurisdiction: GB -\u003e RU",
                                                       "rule":  "SANCTIONED_JURISDICTION",
                                                       "side":  "receiver",
                                                       "weight":  0.75
                                                   }
                                               ],
                                      "linked_transaction_ids":  [

                                                                 ],
                                      "occurred_at":  "2026-10-25T17:58:09.011717200Z",
                                      "payment_type":  "WIRE",
                                      "receiver_account":  6100002,
                                      "score":  75,
                                      "sender_account":  6100001,
                                      "transaction_id":  1
                                  },
                  "first_txn_id":  1,
                  "id":  1,
                  "last_txn_id":  1,
                  "resolution_note":  null,
                  "rule_codes":  [
                                     "SANCTIONED_JURISDICTION"
                                 ],
                  "score":  75,
                  "severity":  "high",
                  "status":  "open",
                  "subject_account":  6100002,
                  "txn_count":  1,
                  "updated_at":  "2026-10-25T17:58:09.016036Z"
              },
    "transactions":  [
                         {
                             "amount":  250.0,
                             "amount_gbp":  250.0,
                             "created_at":  "2026-10-25T17:58:09.016036Z",
                             "flagged":  true,
                             "id":  1,
                             "label_laundering":  null,
                             "label_typology":  null,
                             "occurred_at":  "2026-10-25T17:58:09.011717Z",
                             "payment_currency":  "GBP",
                             "payment_type":  "WIRE",
                             "received_currency":  "GBP",
                             "receiver_account":  6100002,
                             "receiver_country":  "RU",
                             "risk_score":  75,
                             "rule_codes":  [
                                                "SANCTIONED_JURISDICTION"
                                            ],
                             "sender_account":  6100001,
                             "sender_country":  "GB"
                         }
                     ]
}


id              : 1
dedupe_key      : 6100002:20261025:SANCTIONED_JURISDICTION
subject_account : 6100002
first_txn_id    : 1
last_txn_id     : 1
txn_count       : 1
score           : 75
severity        : high
rule_codes      : {SANCTIONED_JURISDICTION}
explanation     : @{amount_gbp=250.0; hits=System.Object[]; linked_transaction_ids=System.Object[]; 
                  occurred_at=2026-10-25T17:58:09.011717200Z; payment_type=WIRE; receiver_account=6100002; score=75; 
                  sender_account=6100001; transaction_id=1}
status          : under_review
assigned_to     : bob
resolution_note : 
case_id         : 
created_at      : 2026-10-25T17:58:09.016036Z
updated_at      : 2026-10-25T18:01:59.418144Z

alert_ids : {1}
case      : @{assigned_to=; closed_at=; created_at=2026-10-25T18:01:59.433167Z; created_by=bob; description=; id=1; 
            outcome=; priority=high; sar_reference=; status=open; title=Payment to sanctioned jurisdiction; 
            updated_at=2026-10-25T18:01:59.433167Z}


Next, close the case. SAR filing needs the admin key, which
is how the system enforces separation of duties:


id            : 1
title         : Payment to sanctioned jurisdiction
description   : 
status        : closed
priority      : high
assigned_to   : 
sar_reference : SAR-2026-0001
outcome       : sar_filed
created_by    : bob
created_at    : 2026-10-25T18:01:59.433167Z
updated_at    : 2026-10-25T18:08:18.599123Z
closed_at     : 2026-10-25T18:08:18.599123Z

id              : 1
status          : sar_filed
resolution_note : Filed with the FIU




actor action       entity_type entity_id
----- ------       ----------- ---------
mlro  case.close   case        1        
bob   case.create  case        1        
bob   alert.update alert       1  

To see the pattern rules fire, this script sends a layering chain (funds forwarded
hop by hop) and a round trip. Run it in a fresh window, with $h 
set to the ingest key as before:

PS C:\Users\cyber-Willz\Desktop\aml_system> function Send($s,$r,$amt) {
>>   $tx = @{ sender_account=$s; receiver_account=$r; amount=$amt; sender_country="GB"; receiver_country="GB"; payment_type="ACH" } | ConvertTo-Json
>>   $d = Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8080/api/v1/transactions -Headers $h -ContentType "application/json" -Body $tx
>>   "{0} -> {1}  {2,6}  score {3,3}  {4}" -f $s,$r,$amt,$d.risk_score,(($d.hits | % { $_.rule }) -join ", ")
>> }
>> "--- layering chain"; Send 7100001 7100002 25000; Send 7100002 7100003 24800; Send 7100003 7100004 24600; Send 7100004 7100005 24400
>> "--- round trip";     Send 8100001 8100002 9000;  Send 8100002 8100003 9000;  Send 8100003 8100001 9000
--- layering chain
7100001 -> 7100002   25000  score   0  
7100002 -> 7100003   24800  score  30  PASS_THROUGH
7100003 -> 7100004   24600  score  30  PASS_THROUGH
7100004 -> 7100005   24400  score  86  PASS_THROUGH, LAYERING_CHAIN
--- round trip
8100001 -> 8100002    9000  score   0  
8100002 -> 8100003    9000  score  30  PASS_THROUGH
8100003 -> 8100001    9000  score  86  PASS_THROUGH, CIRCULAR_FLOW


docker compose up -d db                       # or any PostgreSQL 14+

$env:DATABASE_URL = "postgres://aml:aml_dev_pw@localhost:5432/aml"
$env:AML_API_KEYS = "adminkey-0123456789:admin:mlro,analystkey-0123456789:analyst:bob,ingestkey-0123456789:ingest:core"

cargo run --release -p aml_api -- migrate
cargo run --release -p aml_api -- import-dataset --format saml-d --file C:\data\SAML-D.csv --limit 600000   # optional demo data

cargo run --release -p aml_api -- import-dataset --format aml-suspicious --file C:\data\AML_Suspicious_Transactions.csv   # + 5,000 KYC profiles
cargo run --release -p aml_api -- import-dataset --format kyc-risk --file 

C:\data\transactions_with_fatf_ofac.csv   # + 2,000 KYC clients, sanctioned ones watchlisted
# formats: saml-d | aml-suspicious | kyc-risk | lending | fraudshield | risk-prediction
cargo run --release -p aml_api -- serve        # http://0.0.0.0:8080
```
`scripts/run_e2e.ps1` does all of the above plus a smoke test.

## How detection works

Each transaction is evaluated against per-account rolling windows (≤256 flows/direction, up to
180 days) held in memory, then persisted. Rule weights combine by noisy-OR into a 0-100 score;
score ≥ `alert_threshold` (default 60) raises an alert. Alerts are de-duplicated per
*subject account × day × primary rule* so one pattern is one alert, not hundreds.

| Rule | Typology |
|---|---|
| `WATCHLIST_HIT` | counterparty on internal/sanctions watchlist |
| `LARGE_TXN` | single large / over-invoicing-scale value |
| `HIGH_RISK_JURISDICTION`, `CURRENCY_CONVERSION` | geography / FX context |
| `FAN_IN`, `FAN_OUT` | many counterparties in a short window, **relative to the account's own history** (`min_focus`) |
| `STRUCTURING_HUB` | slow fan-in of sub-threshold amounts from many senders |
| `PASS_THROUGH`, `LAYERING_CHAIN` | funds forwarded almost in full within hours; chains of such hops |
| `CIRCULAR_FLOW` | round-tripping through a 3-5 hop loop |
| `REPEATED_PAIR`, `CASH_VELOCITY` | smurfing / drip cash movements |
| `BEHAVIOUR_CHANGE`, `DORMANT_REACTIVATION` | deviation from the account's own pattern |
| `HIGH_RISK_CUSTOMER`, `CUSTOMER_JURISDICTION` | KYC rating / PEP; high-risk customer **in** a very-high-risk jurisdiction (62% suspicious in the benchmark vs 3% for either alone) |
| `SANCTIONED_JURISDICTION` | counterparty in a sanctioned jurisdiction (alerts alone); sender's own country counts at half weight |
| `STRUCTURING_BAND` | payment within 10% below the reporting threshold **and** out of pattern for the sender; repeats within 7 days strengthen it |
| `TRADE_MISPRICING` | large payment (>= 50k) from an import/export customer |
| `INCOME_MISMATCH` | single amount ≥ 50% of declared annual income, or 30-day outflow ≥ 100% (unvalidated on labelled AML data) |

When a pattern completes (e.g. the 6th sender of a fan-in), the *earlier legs* are linked
retroactively and pick up that rule's weight. All rules, weights, parameters, the alert threshold
and jurisdiction lists are editable at runtime (`PUT /api/v1/rules/{code}`, `/settings`,
`/jurisdictions/{cc}`); changes apply to subsequently scored transactions and are audited.

## API (`/api/v1`, header `x-api-key`)

| Method & path | Role |
|---|---|
| `POST /transactions`, `POST /transactions/batch` (≤5000) | ingest / admin |
| `GET /transactions`, `/transactions/{id}`, `/accounts`, `/accounts/{n}`, `/accounts/{n}/transactions`, `/accounts/{n}/network` | any |
| `GET /alerts`, `/alerts/{id}`; `PATCH /alerts/{id}` | read: any · update: analyst/admin |
| `POST /cases`, `GET /cases[/{id}]`, `PATCH /cases/{id}`, `POST /cases/{id}/notes`, `/alerts` | analyst/admin |
| `POST /cases/{id}/close` (`no_action` or `sar_filed` + `sar_reference`) | analyst · **sar_filed: admin** |
| `PUT /customers/{ref}` (KYC rating, PEP, income, linked accounts), `GET /customers[/{ref}]` | analyst/admin · read: any |
| `POST /rules/import` (apply an `aml-eval --fit-out` calibration, all-or-nothing) | admin |
| `GET /rules`, `PUT /rules/{code}`, `PUT /settings`, `PUT /jurisdictions/{cc}`, `POST/DELETE /watchlist` | admin |
| `GET /stats/overview`, `GET /audit`, `GET /health` (no auth) | any |

Alert workflow: `open → under_review → escalated → sar_filed | closed_confirmed | closed_false_positive`;
closing requires a resolution note; reopening a closed alert is admin-only.
Transaction body: `sender_account, receiver_account, amount, payment_currency, received_currency,
sender_country (ISO-2), receiver_country, payment_type, [occurred_at], [amount_gbp]`.

## Measured performance (SAML-D, 9.5M labelled transactions)

Back-test with `aml-eval --csv SAML-D.csv` (first 14 days treated as warm-up; 9.1M scored,
9,393 laundering transactions, base rate 0.10%). Engine speed ≈ 180k tx/s; full pipeline with
PostgreSQL persistence ≈ 16k tx/s on a single shared core.

| Alert threshold | Precision | Recall | Alerts raised |
|---|---|---|---|
| 50 | 1.2% | 32% | 262,843 |
| **60 (default)** | **8.6%** | **18.8%** | **20,509** |
| 70 | 10.4% | 8.8% | 7,929 |

Rule lift vs. base rate: STRUCTURING_HUB ≈ 250×, CIRCULAR_FLOW / LAYERING_CHAIN ≈ 800-1000×, LARGE_TXN 9×,
FAN_IN 7×, REPEATED_PAIR 7×. Recall by typology at threshold 50: Structuring 78%, Fan_In 77%,
Over-Invoicing 100%, Single_large 47%, Smurfing 45%, Cash_Withdrawal 43%, Cycle 30%; **weak:** Deposit-Send
(3%), Layered_Fan_Out (0.4%), Bipartite (1%), Behavioural_Change (0%).

**Read these numbers honestly.** SAML-D is synthetic, and its normal traffic deliberately contains
heavy fan-in/out, so naive count rules do worse here than on real portfolios; conversely real data
has behaviours the generator lacks. Transaction-level precision is low by nature at a 0.1% base
rate — alert de-duplication and the analyst workflow are what make volume workable. Treat the
default weights as a calibrated starting point, re-tune on your own labelled history, and
validate against your regulator's expectations before relying on it for compliance.

## Using the extra datasets (v0.2 and v0.3)

v0.3 adds the sanctioned-jurisdiction tier, three rules, sector/PEP/sanctions KYC attributes and the `kyc-risk` / `lending` adapters; see the
"Round 2" section of **DATASETS.md** (including two regressions the data exposed and how they were fixed). Upgrade with `aml migrate`.

### v0.2 notes

See **DATASETS.md** for what each of the nine datasets showed. Headlines: only AML_Suspicious carried real signal (jurisdiction ×
KYC rating), which produced the customer layer and the `very_high` jurisdiction tier; FraudShield and Risk_prediction are noise and are
used as false-positive controls (0 alerts); credit datasets only sanity-checked the income rule.
Calibrate to your own data: `aml-eval --format aml-suspicious --csv X --fit-out tuned.json --eval-from 0.7`
(fit on the first 70%, report on the unseen last 30%), then `POST /api/v1/rules/import`.
On the AML_Suspicious holdout this moved F1 from 0.49 to 0.72.
Upgrading from v0.1: `aml migrate` (adds `customers`, `accounts.customer_id`, 4 payment types, the `very_high` tier) preserves existing data.

## Design notes & known limits

* Ingestion is serialised under one engine lock (score + persist are atomic w.r.t. state). If a
  DB write fails the engine is rebuilt from the database, so memory never runs ahead of storage.
  Horizontal scale-out would need account-sharded engines.
* On start the engine rehydrates from the last `AML_HYDRATE_DAYS` of stored transactions.
* The FX table in `default_config()` is a **placeholder**; supply `amount_gbp` or real rates.
* Default high-risk jurisdictions are a seed derived from the benchmark data, **not** an official
  FATF list — replace them with your own risk assessment.
* Time-ordered input is assumed within a window (batches are sorted internally); heavily
  back-dated events degrade window accuracy.
* Out of scope: sanctions-list name screening, KYC onboarding workflow (ratings are stored and used, not derived), SAR document generation
  and regulator submission, rescoring history after rule changes, TLS (terminate at a proxy).
