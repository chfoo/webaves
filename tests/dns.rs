use webaves::net::dns::Builder;

#[test_log::test(tokio::test)]
#[ignore = "external resources"]
async fn test_resolver() {
    let mut resolver = Builder::new()
        .with_doh_server("1.1.1.1".parse().unwrap(), 443, "cloudflare-dns.com")
        .with_doh_server("8.8.8.8".parse().unwrap(), 443, "dns.google")
        .build();

    let result = resolver.lookup_address("www.icanhascheezburger.com").await;
    assert!(matches!(result, Ok(_)));

    let lookup = result.unwrap();
    assert!(!lookup.addresses.is_empty());
    assert!(lookup.records.is_some());

    assert!(matches!(
        resolver
            .lookup_address(&webaves::net::dns::random_domain())
            .await,
        Err(webaves::net::dns::ResolverError::NotFound)
    ));
}
