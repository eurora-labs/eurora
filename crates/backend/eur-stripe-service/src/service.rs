use std::str::FromStr;

use crate::{
    config::Config,
    error::{Result, StripeServiceError},
    types::*,
};
use chrono::{DateTime, Utc};
use eur_remote_db::DatabaseManager;
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCustomer, CreatePaymentIntent, CreatePrice,
    CreateProduct, CreateSubscription, Currency, Customer, PaymentIntent, Price, Product,
    Subscription,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct StripeService {
    client: Client,
    pub db: DatabaseManager,
    pub config: Config,
}

impl StripeService {
    pub fn new(config: Config, db: DatabaseManager) -> Self {
        let client = Client::new(&config.stripe.secret_key);
        Self { client, db, config }
    }

    // Customer management
    pub async fn create_customer(
        &self,
        request: CreateCustomerRequest,
    ) -> Result<CustomerResponse> {
        let mut create_customer = CreateCustomer::new();
        create_customer.email = Some(&request.email);

        if let Some(name) = &request.name {
            create_customer.name = Some(name);
        }

        if let Some(description) = &request.description {
            create_customer.description = Some(description);
        }

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_customer.metadata = Some(metadata_map);
            }
        }

        let customer = Customer::create(&self.client, create_customer).await?;

        // Store customer in database if we have a user_id in metadata
        if let Some(metadata) = &customer.metadata {
            if let Some(user_id_str) = metadata.get("user_id") {
                if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                    let update_request = eur_remote_db::UpdateUserStripeCustomerRequest {
                        stripe_customer_id: customer.id.to_string(),
                    };
                    let _ = self
                        .db
                        .update_user_stripe_customer(user_id, update_request)
                        .await;
                }
            }
        }

        Ok(CustomerResponse {
            id: customer.id.to_string(),
            email: customer.email.unwrap_or_default(),
            name: customer.name,
            description: customer.description,
            created: DateTime::from_timestamp(
                customer
                    .created
                    .expect("Customer created timestamp is missing"),
                0,
            )
            .unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&customer.metadata).unwrap_or_default(),
        })
    }

    pub async fn get_customer(&self, customer_id: &str) -> Result<CustomerResponse> {
        let customer_id = customer_id.parse().map_err(|_| {
            StripeServiceError::InvalidRequest("Invalid customer ID format".to_string())
        })?;

        let customer = Customer::retrieve(&self.client, &customer_id, &[]).await?;

        Ok(CustomerResponse {
            id: customer.id.to_string(),
            email: customer.email.unwrap_or_default(),
            name: customer.name,
            description: customer.description,
            created: DateTime::from_timestamp(
                customer
                    .created
                    .expect("Customer created timestamp is missing"),
                0,
            )
            .unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&customer.metadata).unwrap_or_default(),
        })
    }

    // Product management
    pub async fn create_product(&self, request: CreateProductRequest) -> Result<ProductResponse> {
        let mut create_product = CreateProduct::new(&request.name);

        if let Some(description) = &request.description {
            create_product.description = Some(description);
        }

        create_product.active = request.active;

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_product.metadata = Some(metadata_map);
            }
        }

        let product = Product::create(&self.client, create_product).await?;

        // Store product in database
        let db_request = eur_remote_db::CreateProductRequest {
            stripe_product_id: product.id.to_string(),
            name: product
                .name
                .clone()
                .expect("Product name is missing")
                .clone(),
            description: product.description.clone(),
            active: Some(product.active.unwrap_or(true)),
            metadata: Some(serde_json::to_value(&product.metadata).unwrap_or_default()),
        };
        let _ = self.db.create_product(db_request).await;

        Ok(ProductResponse {
            id: product.id.to_string(),
            name: product
                .name
                .clone()
                .expect("Product name is missing")
                .clone(),
            description: product.description,
            active: product.active.unwrap_or(true),
            created: DateTime::from_timestamp(
                product
                    .created
                    .expect("Product created timestamp is missing"),
                0,
            )
            .unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&product.metadata).unwrap_or_default(),
        })
    }

    pub async fn get_product(&self, product_id: &str) -> Result<ProductResponse> {
        let product_id = product_id.parse().map_err(|_| {
            StripeServiceError::InvalidRequest("Invalid product ID format".to_string())
        })?;

        let product = Product::retrieve(&self.client, &product_id, &[]).await?;

        Ok(ProductResponse {
            id: product.id.to_string(),
            name: product
                .name
                .clone()
                .expect("Product name is missing")
                .clone(),
            description: product.description,
            active: product.active.unwrap_or(true),
            created: DateTime::from_timestamp(
                product
                    .created
                    .expect("Product created timestamp is missing"),
                0,
            )
            .unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&product.metadata).unwrap_or_default(),
        })
    }

    // Price management
    pub async fn create_price(&self, request: CreatePriceRequest) -> Result<PriceResponse> {
        let currency: Currency = request
            .currency
            .parse()
            .map_err(|_| StripeServiceError::InvalidRequest("Invalid currency".to_string()))?;

        let mut create_price = CreatePrice::new(currency);
        create_price.product = Some(stripe::IdOrCreate::Id(&request.product_id));
        create_price.unit_amount = Some(request.unit_amount);

        if let Some(recurring) = &request.recurring {
            let interval = match recurring.interval.as_str() {
                "day" => stripe::CreatePriceRecurringInterval::Day,
                "week" => stripe::CreatePriceRecurringInterval::Week,
                "month" => stripe::CreatePriceRecurringInterval::Month,
                "year" => stripe::CreatePriceRecurringInterval::Year,
                _ => {
                    return Err(StripeServiceError::InvalidRequest(
                        "Invalid recurring interval".to_string(),
                    ));
                }
            };
            let mut recurring_params = stripe::CreatePriceRecurring {
                interval,
                interval_count: None,
                aggregate_usage: None,
                usage_type: None,
                trial_period_days: None,
            };
            if let Some(interval_count) = recurring.interval_count {
                recurring_params.interval_count = Some(interval_count as u64);
            }
            create_price.recurring = Some(recurring_params);
        }

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_price.metadata = Some(metadata_map);
            }
        }

        let price = Price::create(&self.client, create_price).await?;

        // Get product from database to link price
        if let Ok(product) = self.db.get_product_by_stripe_id(&request.product_id).await {
            let db_request = eur_remote_db::CreatePriceRequest {
                stripe_price_id: price.id.to_string(),
                product_id: product.id,
                active: Some(price.active.unwrap_or(true)),
                currency: price.currency.expect("Currency is missing").to_string(),
                unit_amount: price.unit_amount,
                recurring_interval: price.recurring.as_ref().map(|r| r.interval.to_string()),
                recurring_interval_count: price.recurring.as_ref().map(|r| r.interval_count as i32),
                billing_scheme: Some("per_unit".to_string()),
                tiers: None,
                metadata: Some(serde_json::to_value(&price.metadata).unwrap_or_default()),
            };
            let _ = self.db.create_price(db_request).await;
        }

        Ok(PriceResponse {
            id: price.id.to_string(),
            product_id: request.product_id,
            unit_amount: price.unit_amount.unwrap_or(0),
            currency: price.currency.expect("Currency is missing").to_string(),
            recurring: price.recurring.map(|r| RecurringResponse {
                interval: r.interval.to_string(),
                interval_count: r.interval_count as i32,
            }),
            active: price.active.unwrap_or(true),
            created: DateTime::from_timestamp(
                price.created.expect("Price created timestamp is missing"),
                0,
            )
            .unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&price.metadata).unwrap_or_default(),
        })
    }

    // Subscription management
    pub async fn create_subscription(
        &self,
        request: CreateSubscriptionRequest,
    ) -> Result<SubscriptionResponse> {
        let customer_id = stripe::CustomerId::from_str(&request.customer_id).map_err(|_| {
            StripeServiceError::InvalidRequest("Invalid customer ID format".to_string())
        })?;
        let mut create_subscription = CreateSubscription::new(customer_id);
        create_subscription.items = Some(vec![stripe::CreateSubscriptionItems {
            price: Some(request.price_id.clone()),
            quantity: request.quantity.map(|q| q as u64),
            ..Default::default()
        }]);

        if let Some(trial_days) = request.trial_period_days {
            create_subscription.trial_period_days = Some(trial_days as u32);
        }

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_subscription.metadata = Some(metadata_map);
            }
        }

        let subscription = Subscription::create(&self.client, create_subscription).await?;

        // Store subscription in database
        if let Ok(user) = self
            .db
            .get_user_by_stripe_customer_id(&request.customer_id)
            .await
        {
            let db_request = eur_remote_db::CreateSubscriptionRequest {
                stripe_subscription_id: subscription.id.to_string(),
                user_id: user.id,
                status: subscription.status.to_string(),
                current_period_start: DateTime::from_timestamp(
                    subscription.current_period_start,
                    0,
                )
                .unwrap_or_else(Utc::now),
                current_period_end: DateTime::from_timestamp(subscription.current_period_end, 0)
                    .unwrap_or_else(Utc::now),
                cancel_at_period_end: Some(subscription.cancel_at_period_end),
                canceled_at: subscription
                    .canceled_at
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
                ended_at: subscription
                    .ended_at
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
                trial_start: subscription
                    .trial_start
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
                trial_end: subscription
                    .trial_end
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
                metadata: Some(serde_json::to_value(&subscription.metadata).unwrap_or_default()),
            };
            let _ = self.db.create_subscription(db_request).await;
        }

        Ok(SubscriptionResponse {
            id: subscription.id.to_string(),
            customer_id: request.customer_id,
            status: subscription.status.to_string(),
            current_period_start: DateTime::from_timestamp(subscription.current_period_start, 0)
                .unwrap_or_else(Utc::now),
            current_period_end: DateTime::from_timestamp(subscription.current_period_end, 0)
                .unwrap_or_else(Utc::now),
            trial_start: subscription
                .trial_start
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
            trial_end: subscription
                .trial_end
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)),
            cancel_at_period_end: subscription.cancel_at_period_end,
            created: DateTime::from_timestamp(subscription.created, 0).unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&subscription.metadata).unwrap_or_default(),
        })
    }

    // Payment Intent management
    pub async fn create_payment_intent(
        &self,
        request: CreatePaymentIntentRequest,
    ) -> Result<PaymentIntentResponse> {
        let currency: Currency = request
            .currency
            .parse()
            .map_err(|_| StripeServiceError::InvalidRequest("Invalid currency".to_string()))?;

        let mut create_payment_intent = CreatePaymentIntent::new(request.amount, currency);

        if let Some(customer_id) = &request.customer_id {
            create_payment_intent.customer =
                Some(stripe::CustomerId::from_str(customer_id).unwrap());
        }

        if let Some(description) = &request.description {
            create_payment_intent.description = Some(description);
        }

        if request.automatic_payment_methods.unwrap_or(true) {
            create_payment_intent.automatic_payment_methods =
                Some(stripe::CreatePaymentIntentAutomaticPaymentMethods {
                    enabled: true,
                    ..Default::default()
                });
        }

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_payment_intent.metadata = Some(metadata_map);
            }
        }

        let payment_intent = PaymentIntent::create(&self.client, create_payment_intent).await?;

        Ok(PaymentIntentResponse {
            id: payment_intent.id.to_string(),
            amount: payment_intent.amount,
            currency: payment_intent.currency.to_string(),
            status: payment_intent.status.to_string(),
            client_secret: payment_intent.client_secret.unwrap_or_default(),
            customer_id: request.customer_id,
            description: payment_intent.description,
            created: DateTime::from_timestamp(payment_intent.created, 0).unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&payment_intent.metadata).unwrap_or_default(),
        })
    }

    // Checkout Session management
    pub async fn create_checkout_session(
        &self,
        request: CreateCheckoutSessionRequest,
    ) -> Result<CheckoutSessionResponse> {
        let mode = match request.mode.as_str() {
            "payment" => CheckoutSessionMode::Payment,
            "setup" => CheckoutSessionMode::Setup,
            "subscription" => CheckoutSessionMode::Subscription,
            _ => {
                return Err(StripeServiceError::InvalidRequest(
                    "Invalid mode".to_string(),
                ));
            }
        };

        let line_items: Vec<CreateCheckoutSessionLineItems> = request
            .line_items
            .into_iter()
            .map(|item| CreateCheckoutSessionLineItems {
                price: Some(item.price_id),
                quantity: Some(item.quantity as u64),
                ..Default::default()
            })
            .collect();

        let mut create_session = CreateCheckoutSession::new();
        create_session.mode = Some(mode);
        create_session.line_items = Some(line_items);
        create_session.success_url = Some(&request.success_url);
        create_session.cancel_url = Some(&request.cancel_url);

        if let Some(customer_id) = &request.customer_id {
            create_session.customer = Some(stripe::CustomerId::from_str(customer_id).unwrap());
        } else if let Some(email) = &request.customer_email {
            create_session.customer_email = Some(email);
        }

        if let Some(metadata) = &request.metadata {
            if let Ok(metadata_map) = serde_json::from_value::<
                std::collections::HashMap<String, String>,
            >(metadata.clone())
            {
                create_session.metadata = Some(metadata_map);
            }
        }

        let session = CheckoutSession::create(&self.client, create_session).await?;

        Ok(CheckoutSessionResponse {
            id: session.id.to_string(),
            url: session.url,
            customer_id: request.customer_id,
            mode: request.mode,
            status: session.status.map(|s| s.to_string()).unwrap_or_default(),
            success_url: request.success_url,
            cancel_url: request.cancel_url,
            created: DateTime::from_timestamp(session.created, 0).unwrap_or_else(Utc::now),
            metadata: serde_json::to_value(&session.metadata).unwrap_or_default(),
        })
    }
}
