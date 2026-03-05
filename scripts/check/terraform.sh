#!/usr/bin/env bash
# Terraform リント（fmt チェック + validate + tflint）
# validate は terraform init 済みの環境のみ実行（init にはプロバイダーダウンロードが必要）
set -euo pipefail

echo "Terraform fmt check..."
terraform fmt -check -recursive infra/terraform/

# .tf ファイルが存在する環境ディレクトリで validate と tflint を実行
for dir in infra/terraform/environments/*/; do
    if ls "$dir"*.tf &>/dev/null; then
        if [ -d "${dir}.terraform" ]; then
            echo "Terraform validate: $dir"
            (cd "$dir" && terraform validate)
        else
            echo "Skip validate: $dir（terraform init 未実行。CI では自動実行されます）"
        fi
        echo "TFLint: $dir"
        (cd "$dir" && tflint --config "$(pwd)/../../.tflint.hcl" --init && tflint --config "$(pwd)/../../.tflint.hcl")
    fi
done
