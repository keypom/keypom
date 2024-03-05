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
  { question: "Did you attend previously?", required: false },
];
export const artworkUrls: string[] = [
  "https://media.istockphoto.com/id/1001928116/photo/empty-vintage-seat-in-auditorium-or-theater-with-lights-on-stage.webp?s=612x612&w=is&k=20&c=EOlmMqR60cPQILZVrIXIntn24z-k3QlJMkERKqzgPWY=",
  "https://media.istockphoto.com/id/1415056757/photo/colorful-wedding-tents-at-night.webp?s=612x612&w=is&k=20&c=cLRyUIT40mZE5YNSvFCRys1BVh0nXGA_mjwuVq-R5Kw=",
  "https://images.unsplash.com/photo-1492684223066-81342ee5ff30?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8Mnx8ZXZlbnR8ZW58MHx8MHx8fDA%3D",
  "https://images.unsplash.com/photo-1511795409834-ef04bbd61622?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8Nnx8ZXZlbnR8ZW58MHx8MHx8fDA%3D",
  "https://images.unsplash.com/photo-1511578314322-379afb476865?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8MTR8fGV2ZW50fGVufDB8fDB8fHww",
  "https://images.unsplash.com/photo-1522158637959-30385a09e0da?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8MTV8fGV2ZW50fGVufDB8fDB8fHww",
  "https://plus.unsplash.com/premium_photo-1686783009584-0ef0afc5fb5b?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8MjV8fGV2ZW50fGVufDB8fDB8fHww",
  "https://images.unsplash.com/photo-1470299067034-07c696e9ef07?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8N3x8Zml0bmVzcyUyMGV2ZW50fGVufDB8fDB8fHww",
  "https://images.unsplash.com/photo-1518619745898-93e765966dcd?w=800&auto=format&fit=crop&q=60&ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxzZWFyY2h8MTR8fGZpdG5lc3MlMjBldmVudHxlbnwwfHwwfHx8MA%3D%3D",
];

export const ticketArtworkUrls: string[] = [
  "https://thumbs.dreamstime.com/z/admission-ticket-4293622.jpg",
  "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcRuIEwUr6AlRkisoxLUvqQsqlIgyI3cqyIuxfIa9SfhdL6bQQFUpiQKrUSbQSnqzO8BDKw&usqp=CAU",
  "https://previews.123rf.com/images/iqoncept/iqoncept2002/iqoncept200200040/140781657-exclusive-admission-ticket-vip-pass-special-limited-access-3d-illustration.jpg",
];
