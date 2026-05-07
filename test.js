const text = "

---
*Harnessed \uD83D\uDCCB model \"id\" \u00B7 session \"01KR1RRNZAMND4X2N3EN\" \u00B7 entry #0 \u00B7 prev \"prev\"*\"; const regex = /\n\n---\n\*Harnessed.*session \([A-Z0-9]{26})\.*\*/; console.log(text.match(regex));