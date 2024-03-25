import { QuestionInfo } from "./interfaces";

// Data for variation
export const eventThemes: string[] = [
  "Music",
  "Tech",
  "Art",
  "Food",
  "Film",
  "Literature",
  "Adventure",
  "Fitness",
];
export const locations: string[] = [
  "New York",
  "California",
  "Paris",
  "London",
  "Los Angeles",
  "Singapore",
  "Berlin",
  "Tokyo",
];
export const ticketTypes: string[] = [
  "VIP",
  "Ground",
  "All-Day",
  "Platinum",
  "Early Bird",
  "General Admission",
  "Exclusive",
  "Backstage",
];
export const descriptions: { [key: string]: string } = {
  VIP: "Enjoy exclusive access with priority seating, backstage tours, and meet-and-greets.",
  Ground: "Access to all ground events, perfect for those who love to explore.",
  "All-Day":
    "Spend the entire day at the event with access to all general areas.",
  Platinum: "The ultimate experience with all-access passes and premium perks.",
  "Early Bird":
    "Special pricing for early purchasers, with the same benefits as General Admission.",
  "General Admission":
    "Standard access to the event, a great way to enjoy the day.",
  Exclusive:
    "Limited tickets offering private sessions and unique experiences.",
  Backstage:
    "Behind-the-scenes access with a chance to meet the performers or speakers.",
};
export const questions: QuestionInfo[] = [
  { question: "First Name", required: true },
  { question: "Last Name", required: true },
  { question: "How did you find out about this event?", required: true },
  {
    question: "How many events have you attended in the past year?",
    required: false,
  },
  { question: "How many people are in your company?", required: false },
];
export const artworkUrls: string[] = [
  "bafkreiate6gzrw3sd4qom6zgo7cfxpjoxgtri2qhcsg5devfbt7miw44fm",
  "bafybeiehk3mzsj2ih4u4fkvmkfrome3kars7xyy3bxh6xfjquws4flglqa",
  "bafybeifzu62j4pydfymog3mdnxtxeez5t4ndkuwm3wfu6z2kh5xryuz7u4",
  "bafybeibr5uoixk6ywwlacntniyuepkb4bedwgghtstg3q3vusp6u6z5a6q",
  "bafybeibdrggqmhfrnq3eogc6a5iffujx2fnsz55r5rl3tzda3ycvjivj7q",
  "bafkreif753krqnh5dzoroqcsbht4oucoh7fmpp327y3stdbnogilo26lo4",
  "bafybeiblargpzhwxgmbzzci6n6oubfhcw33cdqb4uqx62sxrvf5biwcszi",
  "bafkreifuuae4uzclz5futlfqrq43aqk6peb26er6dz7nhrserr6f7zqrqy",
  "bafybeiax2n6wtil67a6w5qcdm4jwnnxb34ujy2ldgbbanpaoudv7jvgizu",
  "bafkreiaadsk6v5nygmgiwz2lfukdpa2mqdlsoq5lhnjibjjxsatwcfflzq",
  "bafkreifgjnfpzjpfijndodzqw262z2xrec3qjfut5nyoekbysozwwpqakq",
  "bafybeihnb36l3xvpehkwpszthta4ic6bygjkyckp5cffxvszbcltzyjcwi",
  "bafybeibowqrxndtkthbzvsgxmggdq6tvoanzzu5wld7pcru7bawf7vi6ue",
  "bafybeicek3skoaae4p5chsutjzytls5dmnj5fbz6iqsd2uej334sy46oge",
];

export const ticketArtworkUrls: string[] = [
  "bafybeiehk3mzsj2ih4u4fkvmkfrome3kars7xyy3bxh6xfjquws4flglqa",
  "bafybeifzu62j4pydfymog3mdnxtxeez5t4ndkuwm3wfu6z2kh5xryuz7u4",
  "bafybeibr5uoixk6ywwlacntniyuepkb4bedwgghtstg3q3vusp6u6z5a6q",
  "bafybeibdrggqmhfrnq3eogc6a5iffujx2fnsz55r5rl3tzda3ycvjivj7q",
  "bafkreif753krqnh5dzoroqcsbht4oucoh7fmpp327y3stdbnogilo26lo4",
  "bafybeiblargpzhwxgmbzzci6n6oubfhcw33cdqb4uqx62sxrvf5biwcszi",
  "bafkreifuuae4uzclz5futlfqrq43aqk6peb26er6dz7nhrserr6f7zqrqy",
  "bafybeiax2n6wtil67a6w5qcdm4jwnnxb34ujy2ldgbbanpaoudv7jvgizu",
  "bafkreiaadsk6v5nygmgiwz2lfukdpa2mqdlsoq5lhnjibjjxsatwcfflzq",
  "bafkreifgjnfpzjpfijndodzqw262z2xrec3qjfut5nyoekbysozwwpqakq",
  "bafybeihnb36l3xvpehkwpszthta4ic6bygjkyckp5cffxvszbcltzyjcwi",
  "bafybeibowqrxndtkthbzvsgxmggdq6tvoanzzu5wld7pcru7bawf7vi6ue",
  "bafybeicek3skoaae4p5chsutjzytls5dmnj5fbz6iqsd2uej334sy46oge",
];
