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

curl -X GET \
  https://api.public.com/userapigateway/trading/instruments \
  -H "Authorization: Bearer $PUBLIC_API_KEY" \
  > instruments.json
```

## Public API

[Official Docs](https://public.com/api/docs)

### Implementation coverage

**Authorization**

- [x] Create access token

**List Accounts**

- [x] Get accounts

**Account Details**

- [x] Get Account portfolio v2
- [ ] Get history

**Instrument Details**

- [ ] Get all instruments
- [ ] Get instrument

**Market Data**

- [x] Get quotes
- [x] Get option expirations
- [x] Get option chain

**Order Placement**

- [ ] Preflight single leg
- [ ] Preflight multi leg
- [ ] Place order
- [ ] Place multileg order
- [ ] Get order
- [ ] Cancel order

**Option Details**

- [WIP] Get option greeks
