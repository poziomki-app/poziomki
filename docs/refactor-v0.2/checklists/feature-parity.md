# Feature Parity Checklist

All features from the original app must work in the new app before release.

## Auth Features

- [ ] Email + OTP login works
- [ ] Session persists across app restart
- [ ] Auto-login on app start
- [ ] Logout clears all local data
- [ ] Account deletion removes all data (GDPR)
- [ ] Data export downloads complete JSON (GDPR)

## Profile Features

- [ ] Discovery feed loads
- [ ] Discovery feed scrolls smoothly (60fps)
- [ ] Discovery feed pull-to-refresh
- [ ] Profile detail shows all fields
- [ ] Profile photos display (multiple)
- [ ] Profile photo gallery (swipe through)
- [ ] Edit profile screen works
- [ ] Edit profile saves changes
- [ ] Photo upload works (camera)
- [ ] Photo upload works (gallery)
- [ ] Photo reorder works
- [ ] Photo delete works
- [ ] Tags selection works
- [ ] Degree autocomplete works
- [ ] Bookmarks save profiles
- [ ] Bookmarks list displays
- [ ] Matching shows compatible profiles first

## Event Features

- [ ] Events list loads
- [ ] Events list scrolls smoothly
- [ ] Events list pull-to-refresh
- [ ] Time filter: Today
- [ ] Time filter: This week
- [ ] Time filter: This month
- [ ] Time filter: All
- [ ] Event detail shows all info
- [ ] Event cover image displays
- [ ] Event date/time displays correctly
- [ ] Event location displays
- [ ] Create event screen works
- [ ] Create event form validation
- [ ] Create event date/time picker
- [ ] Create event cover image upload
- [ ] Edit event updates correctly
- [ ] Attend event (join)
- [ ] Leave event
- [ ] Attendee list displays
- [ ] Tap attendee opens profile
- [ ] Event chat accessible (attendees only)
- [ ] Non-attendees cannot access event chat

## Chat Features (All Must Work)

### Conversations
- [ ] Conversations list loads
- [ ] Conversations sorted by last message
- [ ] Unread count badges display
- [ ] Conversation shows last message preview
- [ ] Conversation shows participant name/photo
- [ ] Personal chats work
- [ ] Event chats work

### Messages
- [ ] Messages load with pagination
- [ ] Messages scroll smoothly (60fps)
- [ ] Load more on scroll up
- [ ] Send text message
- [ ] Message appears instantly (local-first)
- [ ] Message syncs to server
- [ ] Message syncs to other devices
- [ ] Timestamps display correctly
- [ ] Own messages styled differently
- [ ] Their messages styled differently

### Encryption
- [ ] Messages encrypted before send
- [ ] Messages decrypted on receive
- [ ] E2E badge displays
- [ ] Server cannot read messages

### Reactions
- [ ] Add reaction to message
- [ ] Remove own reaction
- [ ] Toggle reaction (add if not present, remove if present)
- [ ] Reaction counts display
- [ ] Multiple reaction types on same message
- [ ] Reaction breakdown shows who reacted
- [ ] Tap reaction to see who reacted

### Reply
- [ ] Reply to message
- [ ] Reply preview shows in composer
- [ ] Reply preview shows in message
- [ ] Tap reply scrolls to original

### Edit & Delete
- [ ] Edit own message
- [ ] Edit indicator shows
- [ ] Delete own message
- [ ] Deleted message shows placeholder

### Typing Indicators
- [ ] Typing indicator shows when others type
- [ ] Typing indicator hides after timeout
- [ ] Multiple people typing displays

### Read Receipts
- [ ] Read status updates
- [ ] Shows who has read

### Context Menu
- [ ] Long-press shows context menu
- [ ] Copy text option
- [ ] Reply option
- [ ] React option
- [ ] Edit option (own messages)
- [ ] Delete option (own messages)

### Offline
- [ ] Messages visible offline
- [ ] Can send messages offline
- [ ] Messages queue when offline
- [ ] Messages sync when back online
- [ ] Offline indicator shows
- [ ] Pending count shows

### Images
- [ ] Send image in chat
- [ ] Image thumbnail displays
- [ ] Tap image to view full
- [ ] Image encrypted

## Upload Features

- [ ] Image upload shows progress
- [ ] Image upload can cancel
- [ ] Profile photo upload
- [ ] Event cover upload
- [ ] Chat image upload

## Navigation

- [ ] Tab bar displays
- [ ] Tab bar switches screens
- [ ] Tab bar shows current tab
- [ ] Unread badge on chat tab
- [ ] Back navigation works
- [ ] Modal screens (profile detail, event detail)
- [ ] Modal dismiss gestures

## Visual Parity

- [ ] Colors match original
- [ ] Gradients match original
- [ ] Typography matches
- [ ] Spacing matches
- [ ] Animations feel similar
- [ ] Dark mode only (no light mode)

## Other Features

- [ ] Push notifications receive
- [ ] Push notifications open correct screen
- [ ] Deep links work
- [ ] Privacy policy accessible
- [ ] App handles low memory
- [ ] App handles backgrounding
- [ ] App handles foregrounding

## Performance

- [ ] App startup < 200ms
- [ ] Discovery scroll 60fps
- [ ] Events scroll 60fps
- [ ] Chat scroll 60fps
- [ ] Memory < 100MB
- [ ] APK size < 20MB

## Accessibility

- [ ] Screen reader announces all elements
- [ ] Touch targets ≥ 44x44
- [ ] Contrast meets WCAG AA
- [ ] Reduced motion respected
- [ ] High contrast mode works
