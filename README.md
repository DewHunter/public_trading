# Public Trading

Docs: https://public.com/api/docs

Get a short lived access token:

```Bash
set secret_key <secret key>

curl --request POST \
  --url https://api.public.com/userapiauthservice/personal/access-tokens \
  --header 'Content-Type: application/json' \
  --data "{
    \"validityInMinutes\": 60,
    \"secret\": \"$secret_key\"
  }"

set -x PUBLIC_API_KEY longstring

curl -v -X GET \
https://api.public.com/userapigateway/trading/account \
  -H "Authorization: Bearer $PUBLIC_API_KEY"

curl -v -X GET \
  https://api.public.com/userapigateway/trading/5LI70019/portfolio/v2 \
    -H "Authorization: Bearer $PUBLIC_API_KEY"
```
