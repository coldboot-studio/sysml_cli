# CI/CD integration recipes (for agents)

When the user asks for a CI configuration, pick the recipe that
matches their platform. Each recipe assumes `sysml-validate` is
already installed (binary on PATH inside the runner / image).

For installation strategies (download a tagged release, build from
source, vendor a binary in the image), see TECH_MANUAL.md §3 and
§11.

---

## GitHub Actions

Minimal: validate, upload SARIF to GitHub Advanced Security.

```yaml
name: SysML validation
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

permissions:
  contents: read
  security-events: write   # required to upload SARIF to GHAS

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install sysml-validate
        run: |
          VERSION="0.15.0"
          TARGET="x86_64-unknown-linux-gnu"
          curl -LO "https://github.com/<owner>/sysml-cli/releases/download/v${VERSION}/sysml-validate-${VERSION}-${TARGET}.tar.gz"
          tar -xzf "sysml-validate-${VERSION}-${TARGET}.tar.gz"
          sudo cp "sysml-validate-${VERSION}-${TARGET}/bin/sysml-validate" /usr/local/bin/

      - name: Validate models
        run: sysml-validate validate src --ci > findings.sarif

      - name: Upload SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: findings.sarif
```

**For baseline mode**, replace the validate step:

```yaml
      - name: Validate models (against baseline)
        run: sysml-validate validate src --ci --baseline baseline.sarif > findings.sarif
```

Commit `baseline.sarif` to the repo. Re-seed when intentional drift
occurs.

---

## GitLab CI

```yaml
sysml-validate:
  image: registry.example.com/sysml-validate:0.15.0
  stage: validate
  script:
    - sysml-validate validate src --format junit > junit.xml
  artifacts:
    when: always
    reports:
      junit: junit.xml
```

If GitLab Ultimate with SAST is in play, swap to SARIF:

```yaml
  script:
    - sysml-validate validate src --ci > gl-sast-report.json
  artifacts:
    when: always
    reports:
      sast: gl-sast-report.json
```

(GitLab consumes SARIF under the `sast` report key.)

---

## Jenkins (declarative pipeline)

```groovy
pipeline {
    agent any
    stages {
        stage('SysML validate') {
            steps {
                sh 'sysml-validate validate src --format junit > junit.xml'
            }
            post {
                always {
                    junit 'junit.xml'
                }
            }
        }
    }
}
```

For SARIF + Warnings Next Generation plugin:

```groovy
        stage('SysML validate (SARIF)') {
            steps {
                sh 'sysml-validate validate src --ci > findings.sarif'
            }
            post {
                always {
                    recordIssues tools: [sarif(pattern: 'findings.sarif')]
                }
            }
        }
```

---

## Azure DevOps

```yaml
trigger:
  - main

pool:
  vmImage: ubuntu-latest

steps:
  - script: |
      curl -LO https://github.com/<owner>/sysml-cli/releases/download/v0.15.0/sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
      tar -xzf sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
      sudo cp sysml-validate-0.15.0-x86_64-unknown-linux-gnu/bin/sysml-validate /usr/local/bin/
    displayName: 'Install sysml-validate'

  - script: |
      sysml-validate validate src --ci > $(Build.ArtifactStagingDirectory)/findings.sarif
    displayName: 'Validate SysML models'

  - task: PublishBuildArtifacts@1
    condition: always()
    inputs:
      pathToPublish: '$(Build.ArtifactStagingDirectory)/findings.sarif'
      artifactName: 'sarif'
```

---

## Iron Bank / Platform One

Iron Bank pipelines consume SARIF and feed it into Anchore Enterprise
+ CodeQL. From your pipeline yaml:

```yaml
- name: validate-sysml
  image: registry1.dso.mil/<your-org>/sysml-validate:0.15.0
  commands:
    - sysml-validate validate src --ci > sysml-findings.sarif
  artifacts:
    paths:
      - sysml-findings.sarif
```

Mark `sysml-findings.sarif` as a published artifact so the central
dashboard ingests it.

For the Iron Bank container image itself (a separate piece of work
the maintainer of `sysml-validate` would publish), the base should be
UBI-minimal or distroless, the binary should be cosign-signed in
place, and the image should pass the Iron Bank scanner suite. See
PRD §4 (Phase 4 differentiation candidates).

---

## Pattern: gate + report

A common pattern across platforms: run validation as a strict gate,
but always upload the report even on failure so reviewers can see
what broke.

```sh
sysml-validate validate src --ci > findings.sarif
exit_code=$?

# Always publish the report, even if the gate failed
upload findings.sarif

exit "${exit_code}"
```

In GitHub Actions this is the `if: always()` clause on the upload
step. In Jenkins, it's a `post { always { ... } }` block. Use the
equivalent on whatever platform.

---

## Pattern: strict gate + warning visibility

If the project wants warnings to fail CI:

```sh
sysml-validate validate src --strict --fail-on-warning --ci > findings.sarif
```

If the project wants warnings to *not* fail CI but still be visible
in the report (the common case):

```sh
sysml-validate validate src --strict --ci > findings.sarif
```

The `--strict` flag turns on the unresolved-reference warning
(`SYSML040`). Without `--fail-on-warning`, warnings are reported but
don't change the exit code.

---

## Pattern: official-backend delegation

For deep semantic checks that the native backend can't perform,
delegate to the OMG Pilot Implementation via the official backend.
This is useful as a nightly / pre-release gate, less so as a per-PR
gate (it's slower):

```yaml
  - name: Deep validation (nightly)
    run: |
      sysml-validate validate src --backend official \
        --official-command "sysml-validator --strict {file}" \
        --timeout 300 \
        --ci > deep-findings.sarif
```

The child process is killed after `--timeout` seconds with `SYSML904`
if it hangs. **No shell process is spawned** — the argv template is
parsed and invoked positionally, so shell metacharacters in the
template survive only as literal argv content.
