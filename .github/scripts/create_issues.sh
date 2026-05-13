#!/bin/bash
# Script pour créer les issues GitHub depuis le fichier ISSUES.md
# Nécessite: gh CLI, jq

set -e

REPO="${1:-systm-d/repolens}"
ISSUES_FILE=".github/ISSUES.md"

if [ ! -f "$ISSUES_FILE" ]; then
    echo "Erreur: $ISSUES_FILE non trouvé"
    exit 1
fi

if ! command -v gh &> /dev/null; then
    echo "Erreur: GitHub CLI (gh) n'est pas installé"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "Erreur: jq n'est pas installé"
    exit 1
fi

echo "Création des issues GitHub pour $REPO"
echo "========================================"
echo ""

# Fonction pour extraire un issue du fichier markdown
extract_issue() {
    local issue_num=$1
    local in_issue=false
    local title=""
    local body=""
    local labels=""
    local current_section=""
    
    while IFS= read -r line; do
        if [[ $line =~ ^###\ Issue\ #$issue_num: ]]; then
            in_issue=true
            title=$(echo "$line" | sed 's/^### Issue #[0-9]*: //')
            continue
        fi
        
        if [[ $in_issue == true ]]; then
            if [[ $line =~ ^---$ ]]; then
                break
            fi
            
            if [[ $line =~ ^\*\*Labels:\*\* ]]; then
                labels=$(echo "$line" | sed 's/^\*\*Labels:\*\* //' | tr -d '`')
                continue
            fi
            
            if [[ $line =~ ^\*\*Description:\*\*$ ]] || [[ $line =~ ^\*\*Objectifs:\*\*$ ]] || [[ $line =~ ^\*\*Acceptance\ Criteria:\*\*$ ]]; then
                current_section=$(echo "$line" | sed 's/\*\*//g' | sed 's/:$//')
                body+="$line"$'\n'
                continue
            fi
            
            if [[ -n "$line" ]] || [[ -n "$current_section" ]]; then
                body+="$line"$'\n'
            fi
        fi
    done < "$ISSUES_FILE"
    
    echo "$title|$body|$labels"
}

# Fonction pour créer un issue
create_issue() {
    local issue_num=$1
    local data=$(extract_issue "$issue_num")
    
    if [ -z "$data" ]; then
        echo "Issue #$issue_num non trouvé"
        return 1
    fi
    
    local title=$(echo "$data" | cut -d'|' -f1)
    local body=$(echo "$data" | cut -d'|' -f2)
    local labels=$(echo "$data" | cut -d'|' -f3)
    
    echo "Création de l'issue: $title"
    
    # Créer l'issue
    local issue_json=$(gh issue create \
        --repo "$REPO" \
        --title "$title" \
        --body "$body" \
        --label "$labels" \
        --json number,url,title)
    
    local issue_number=$(echo "$issue_json" | jq -r '.number')
    local issue_url=$(echo "$issue_json" | jq -r '.url')
    
    echo "  ✓ Issue #$issue_number créée: $issue_url"
    echo ""
}

# Demander confirmation
echo "Ce script va créer 55 issues sur $REPO"
read -p "Continuer? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Annulé"
    exit 0
fi

# Créer toutes les issues
for i in {1..55}; do
    create_issue "$i"
    sleep 1  # Éviter le rate limiting
done

echo "Toutes les issues ont été créées!"
