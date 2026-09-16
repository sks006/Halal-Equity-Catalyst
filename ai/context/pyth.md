# Pyth Network Context & Reference

## Overview
Pyth Network delivers institutional-grade real-time market data across equities, crypto, and commodities to Solana and other blockchains.

## Hermes Service
- **Endpoint**: `https://hermes.pyth.network`
- **Latest Price Endpoint**: `/v2/updates/price/latest?ids[]=<FEED_ID>`
- **Response Structure**:
  ```json
  {
    "parsed": [
      {
        "id": "feed_id_hex",
        "price": {
          "price": "12543000000",
          "conf": "1250000",
          "expo": -8,
          "publish_time": 1726456789
        },
        "ema_price": { ... }
      }
    ]
  }
  ```

## Normalization & Math
$$\text{Calculated Price} = \text{price} \times 10^{\text{expo}}$$
$$\text{Confidence Percentage} = \frac{\text{conf}}{\text{price}}$$

Feeds with confidence percentage exceeding 2.0% ($0.02$) must be rejected.
