/// Simplified reverse geocoding using a hardcoded list of major world cities.
/// Matches the nearest city within a 100km radius.

pub struct City {
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
}

/// Haversine distance in km between two (lat, lon) points.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth radius in km
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

/// Look up the nearest city to (lat, lon). Returns `Some("City, Country")` if within 100km.
pub fn reverse_geocode(lat: f64, lon: f64) -> Option<String> {
    let mut best: Option<(&City, f64)> = None;
    for city in CITIES.iter() {
        let dist = haversine_km(lat, lon, city.lat, city.lon);
        if dist <= 100.0 {
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((city, dist));
            }
        }
    }
    best.map(|(c, _)| format!("{}, {}", c.name, c.country))
}

/// Top ~200 world cities by population. Enough for meaningful coverage without bloat.
static CITIES: &[City] = &[
    City { name: "Tokyo", country: "Japan", lat: 35.6762, lon: 139.6503 },
    City { name: "Delhi", country: "India", lat: 28.7041, lon: 77.1025 },
    City { name: "Shanghai", country: "China", lat: 31.2304, lon: 121.4737 },
    City { name: "São Paulo", country: "Brazil", lat: -23.5505, lon: -46.6333 },
    City { name: "Mexico City", country: "Mexico", lat: 19.4326, lon: -99.1332 },
    City { name: "Cairo", country: "Egypt", lat: 30.0444, lon: 31.2357 },
    City { name: "Mumbai", country: "India", lat: 19.0760, lon: 72.8777 },
    City { name: "Beijing", country: "China", lat: 39.9042, lon: 116.4074 },
    City { name: "Dhaka", country: "Bangladesh", lat: 23.8103, lon: 90.4125 },
    City { name: "Osaka", country: "Japan", lat: 34.6937, lon: 135.5023 },
    City { name: "New York", country: "USA", lat: 40.7128, lon: -74.0060 },
    City { name: "Karachi", country: "Pakistan", lat: 24.8607, lon: 67.0011 },
    City { name: "Buenos Aires", country: "Argentina", lat: -34.6037, lon: -58.3816 },
    City { name: "Chongqing", country: "China", lat: 29.4316, lon: 106.9123 },
    City { name: "Istanbul", country: "Turkey", lat: 41.0082, lon: 28.9784 },
    City { name: "Kolkata", country: "India", lat: 22.5726, lon: 88.3639 },
    City { name: "Manila", country: "Philippines", lat: 14.5995, lon: 120.9842 },
    City { name: "Lagos", country: "Nigeria", lat: 6.5244, lon: 3.3792 },
    City { name: "Rio de Janeiro", country: "Brazil", lat: -22.9068, lon: -43.1729 },
    City { name: "Tianjin", country: "China", lat: 39.3434, lon: 117.3616 },
    City { name: "Kinshasa", country: "DR Congo", lat: -4.4419, lon: 15.2663 },
    City { name: "Guangzhou", country: "China", lat: 23.1291, lon: 113.2644 },
    City { name: "Los Angeles", country: "USA", lat: 34.0522, lon: -118.2437 },
    City { name: "Moscow", country: "Russia", lat: 55.7558, lon: 37.6173 },
    City { name: "Shenzhen", country: "China", lat: 22.5431, lon: 114.0579 },
    City { name: "Lahore", country: "Pakistan", lat: 31.5204, lon: 74.3587 },
    City { name: "Bangalore", country: "India", lat: 12.9716, lon: 77.5946 },
    City { name: "Paris", country: "France", lat: 48.8566, lon: 2.3522 },
    City { name: "Bogota", country: "Colombia", lat: 4.7110, lon: -74.0721 },
    City { name: "Jakarta", country: "Indonesia", lat: -6.2088, lon: 106.8456 },
    City { name: "Chennai", country: "India", lat: 13.0827, lon: 80.2707 },
    City { name: "Lima", country: "Peru", lat: -12.0464, lon: -77.0428 },
    City { name: "Bangkok", country: "Thailand", lat: 13.7563, lon: 100.5018 },
    City { name: "Seoul", country: "South Korea", lat: 37.5665, lon: 126.9780 },
    City { name: "Nagoya", country: "Japan", lat: 35.1815, lon: 136.9066 },
    City { name: "Hyderabad", country: "India", lat: 17.3850, lon: 78.4867 },
    City { name: "London", country: "UK", lat: 51.5074, lon: -0.1278 },
    City { name: "Tehran", country: "Iran", lat: 35.6892, lon: 51.3890 },
    City { name: "Chicago", country: "USA", lat: 41.8781, lon: -87.6298 },
    City { name: "Chengdu", country: "China", lat: 30.5728, lon: 104.0668 },
    City { name: "Nanjing", country: "China", lat: 32.0603, lon: 118.7969 },
    City { name: "Wuhan", country: "China", lat: 30.5928, lon: 114.3055 },
    City { name: "Ho Chi Minh City", country: "Vietnam", lat: 10.8231, lon: 106.6297 },
    City { name: "Luanda", country: "Angola", lat: -8.8390, lon: 13.2894 },
    City { name: "Ahmedabad", country: "India", lat: 23.0225, lon: 72.5714 },
    City { name: "Kuala Lumpur", country: "Malaysia", lat: 3.1390, lon: 101.6869 },
    City { name: "Hong Kong", country: "China", lat: 22.3193, lon: 114.1694 },
    City { name: "Hangzhou", country: "China", lat: 30.2741, lon: 120.1551 },
    City { name: "Riyadh", country: "Saudi Arabia", lat: 24.7136, lon: 46.6753 },
    City { name: "Surat", country: "India", lat: 21.1702, lon: 72.8311 },
    City { name: "Houston", country: "USA", lat: 29.7604, lon: -95.3698 },
    City { name: "Pune", country: "India", lat: 18.5204, lon: 73.8567 },
    City { name: "Singapore", country: "Singapore", lat: 1.3521, lon: 103.8198 },
    City { name: "Santiago", country: "Chile", lat: -33.4489, lon: -70.6693 },
    City { name: "Phoenix", country: "USA", lat: 33.4484, lon: -112.0740 },
    City { name: "Philadelphia", country: "USA", lat: 39.9526, lon: -75.1652 },
    City { name: "San Antonio", country: "USA", lat: 29.4241, lon: -98.4936 },
    City { name: "San Diego", country: "USA", lat: 32.7157, lon: -117.1611 },
    City { name: "Dallas", country: "USA", lat: 32.7767, lon: -96.7970 },
    City { name: "San Jose", country: "USA", lat: 37.3382, lon: -121.8863 },
    City { name: "Austin", country: "USA", lat: 30.2672, lon: -97.7431 },
    City { name: "San Francisco", country: "USA", lat: 37.7749, lon: -122.4194 },
    City { name: "Seattle", country: "USA", lat: 47.6062, lon: -122.3321 },
    City { name: "Denver", country: "USA", lat: 39.7392, lon: -104.9903 },
    City { name: "Washington DC", country: "USA", lat: 38.9072, lon: -77.0369 },
    City { name: "Boston", country: "USA", lat: 42.3601, lon: -71.0589 },
    City { name: "Miami", country: "USA", lat: 25.7617, lon: -80.1918 },
    City { name: "Atlanta", country: "USA", lat: 33.7490, lon: -84.3880 },
    City { name: "Toronto", country: "Canada", lat: 43.6532, lon: -79.3832 },
    City { name: "Montreal", country: "Canada", lat: 45.5017, lon: -73.5673 },
    City { name: "Vancouver", country: "Canada", lat: 49.2827, lon: -123.1207 },
    City { name: "Berlin", country: "Germany", lat: 52.5200, lon: 13.4050 },
    City { name: "Madrid", country: "Spain", lat: 40.4168, lon: -3.7038 },
    City { name: "Rome", country: "Italy", lat: 41.9028, lon: 12.4964 },
    City { name: "Milan", country: "Italy", lat: 45.4642, lon: 9.1900 },
    City { name: "Barcelona", country: "Spain", lat: 41.3874, lon: 2.1686 },
    City { name: "Munich", country: "Germany", lat: 48.1351, lon: 11.5820 },
    City { name: "Amsterdam", country: "Netherlands", lat: 52.3676, lon: 4.9041 },
    City { name: "Vienna", country: "Austria", lat: 48.2082, lon: 16.3738 },
    City { name: "Prague", country: "Czech Republic", lat: 50.0755, lon: 14.4378 },
    City { name: "Zurich", country: "Switzerland", lat: 47.3769, lon: 8.5417 },
    City { name: "Warsaw", country: "Poland", lat: 52.2297, lon: 21.0122 },
    City { name: "Budapest", country: "Hungary", lat: 47.4979, lon: 19.0402 },
    City { name: "Brussels", country: "Belgium", lat: 50.8503, lon: 4.3517 },
    City { name: "Stockholm", country: "Sweden", lat: 59.3293, lon: 18.0686 },
    City { name: "Copenhagen", country: "Denmark", lat: 55.6761, lon: 12.5683 },
    City { name: "Oslo", country: "Norway", lat: 59.9139, lon: 10.7522 },
    City { name: "Helsinki", country: "Finland", lat: 60.1699, lon: 24.9384 },
    City { name: "Dublin", country: "Ireland", lat: 53.3498, lon: -6.2603 },
    City { name: "Lisbon", country: "Portugal", lat: 38.7223, lon: -9.1393 },
    City { name: "Athens", country: "Greece", lat: 37.9838, lon: 23.7275 },
    City { name: "Bucharest", country: "Romania", lat: 44.4268, lon: 26.1025 },
    City { name: "Kyiv", country: "Ukraine", lat: 50.4501, lon: 30.5234 },
    City { name: "Saint Petersburg", country: "Russia", lat: 59.9343, lon: 30.3351 },
    City { name: "Sydney", country: "Australia", lat: -33.8688, lon: 151.2093 },
    City { name: "Melbourne", country: "Australia", lat: -37.8136, lon: 144.9631 },
    City { name: "Brisbane", country: "Australia", lat: -27.4698, lon: 153.0251 },
    City { name: "Perth", country: "Australia", lat: -31.9505, lon: 115.8605 },
    City { name: "Auckland", country: "New Zealand", lat: -36.8485, lon: 174.7633 },
    City { name: "Nairobi", country: "Kenya", lat: -1.2921, lon: 36.8219 },
    City { name: "Johannesburg", country: "South Africa", lat: -26.2041, lon: 28.0473 },
    City { name: "Cape Town", country: "South Africa", lat: -33.9249, lon: 18.4241 },
    City { name: "Casablanca", country: "Morocco", lat: 33.5731, lon: -7.5898 },
    City { name: "Addis Ababa", country: "Ethiopia", lat: 9.0250, lon: 38.7469 },
    City { name: "Dar es Salaam", country: "Tanzania", lat: -6.7924, lon: 39.2083 },
    City { name: "Accra", country: "Ghana", lat: 5.6037, lon: -0.1870 },
    City { name: "Dubai", country: "UAE", lat: 25.2048, lon: 55.2708 },
    City { name: "Abu Dhabi", country: "UAE", lat: 24.4539, lon: 54.3773 },
    City { name: "Doha", country: "Qatar", lat: 25.2854, lon: 51.5310 },
    City { name: "Kuwait City", country: "Kuwait", lat: 29.3759, lon: 47.9774 },
    City { name: "Tel Aviv", country: "Israel", lat: 32.0853, lon: 34.7818 },
    City { name: "Beirut", country: "Lebanon", lat: 33.8938, lon: 35.5018 },
    City { name: "Amman", country: "Jordan", lat: 31.9454, lon: 35.9284 },
    City { name: "Baghdad", country: "Iraq", lat: 33.3152, lon: 44.3661 },
    City { name: "Taipei", country: "Taiwan", lat: 25.0330, lon: 121.5654 },
    City { name: "Hanoi", country: "Vietnam", lat: 21.0285, lon: 105.8542 },
    City { name: "Yangon", country: "Myanmar", lat: 16.8661, lon: 96.1951 },
    City { name: "Colombo", country: "Sri Lanka", lat: 6.9271, lon: 79.8612 },
    City { name: "Kathmandu", country: "Nepal", lat: 27.7172, lon: 85.3240 },
    City { name: "Havana", country: "Cuba", lat: 23.1136, lon: -82.3666 },
    City { name: "Quito", country: "Ecuador", lat: -0.1807, lon: -78.4678 },
    City { name: "Montevideo", country: "Uruguay", lat: -34.9011, lon: -56.1645 },
    City { name: "Caracas", country: "Venezuela", lat: 10.4806, lon: -66.9036 },
    City { name: "Honolulu", country: "USA", lat: 21.3069, lon: -157.8583 },
    City { name: "Anchorage", country: "USA", lat: 61.2181, lon: -149.9003 },
    City { name: "Las Vegas", country: "USA", lat: 36.1699, lon: -115.1398 },
    City { name: "Portland", country: "USA", lat: 45.5152, lon: -122.6784 },
    City { name: "Minneapolis", country: "USA", lat: 44.9778, lon: -93.2650 },
    City { name: "Detroit", country: "USA", lat: 42.3314, lon: -83.0458 },
    City { name: "Nashville", country: "USA", lat: 36.1627, lon: -86.7816 },
    City { name: "New Orleans", country: "USA", lat: 29.9511, lon: -90.0715 },
    City { name: "Orlando", country: "USA", lat: 28.5383, lon: -81.3792 },
];
