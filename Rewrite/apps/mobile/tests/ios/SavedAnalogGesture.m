#import "SavedAnalogGesture.h"
#import <math.h>

// Minimal declarations for the installed XCTest runtime, not shipped with the app.
// These private QA APIs may change between Xcode versions; absence fails the test.
@interface NSObject (ThumbleSavedAnalogXCTest)
- (instancetype)initForTouchAtPoint:(CGPoint)point offset:(double)offset;
- (void)liftUpAtOffset:(double)offset;
- (instancetype)initWithName:(NSString *)name interfaceOrientation:(long long)orientation;
- (void)addPointerEventPath:(id)path;
- (id)eventSynthesizer;
- (void)synthesizeEvent:(id)record completion:(void (^)(BOOL, NSError *))completion;
@end

void ThumbleSavedAnalogGesture(XCUIApplication *application, NSArray<NSValue *> *points,
                              CGPoint done, void (^completion)(BOOL)) {
  NSCAssert(NSThread.isMainThread, @"QA gesture must start on its owner thread");
  __block BOOL completed = NO;
  void (^finish)(BOOL) = ^(BOOL succeeded) {
    dispatch_async(dispatch_get_main_queue(), ^{
      if (completed) return;
      completed = YES;
      completion(succeeded);
    });
  };
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_SEC),
                 dispatch_get_main_queue(), ^{ finish(NO); });
  @try {
    CGRect frame = application.frame;
    CGFloat limit = MAX(frame.size.width, frame.size.height);
    CGRect bounds = CGRectMake(0, 0, limit, limit);
    if (points.count != 5 || CGRectIsEmpty(bounds) || bounds.size.width > 2000 ||
        bounds.size.height > 2000 || !isfinite(done.x) || !isfinite(done.y) ||
        !CGRectContainsPoint(bounds, done)) { finish(NO); return; }
    for (NSValue *value in points) {
      if (![value isKindOfClass:NSValue.class] || strcmp(value.objCType, @encode(CGPoint)) != 0) {
        finish(NO); return;
      }
      CGPoint p = value.CGPointValue;
      if (!isfinite(p.x) || !isfinite(p.y) || !CGRectContainsPoint(bounds, p)) {
        finish(NO); return;
      }
    }
    Class pathClass = NSClassFromString(@"XCPointerEventPath");
    Class recordClass = NSClassFromString(@"XCSynthesizedEventRecord");
    if (!pathClass || !recordClass ||
        ![pathClass instancesRespondToSelector:@selector(initForTouchAtPoint:offset:)] ||
        ![pathClass instancesRespondToSelector:@selector(liftUpAtOffset:)] ||
        ![recordClass instancesRespondToSelector:@selector(initWithName:interfaceOrientation:)] ||
        ![recordClass instancesRespondToSelector:@selector(addPointerEventPath:)] ||
        ![XCUIDevice.sharedDevice respondsToSelector:@selector(eventSynthesizer)]) {
      finish(NO); return;
    }
    id synthesizer = [XCUIDevice.sharedDevice eventSynthesizer];
    if (![synthesizer respondsToSelector:@selector(synthesizeEvent:completion:)]) {
      finish(NO); return;
    }
    NSNumber *orientation = [application valueForKey:@"interfaceOrientation"];
    if (![orientation isKindOfClass:NSNumber.class] || orientation.integerValue < 1 ||
        orientation.integerValue > 4) { finish(NO); return; }
    id record = [[recordClass alloc] initWithName:@"Thumble saved analog QA"
                            interfaceOrientation:orientation.longLongValue];
    for (NSUInteger index = 0; index < 5; index++) {
      id path = [[pathClass alloc] initForTouchAtPoint:points[index].CGPointValue offset:0];
      // Release only button finger five, then reuse that slot for Done below.
      [path liftUpAtOffset:index == 4 ? 2.5 : 3.5];
      [record addPointerEventPath:path];
    }
    id dismissal = [[pathClass alloc] initForTouchAtPoint:done offset:2.7];
    [dismissal liftUpAtOffset:2.85];
    [record addPointerEventPath:dismissal];
    [synthesizer synthesizeEvent:record completion:^(BOOL succeeded, NSError *error) {
      finish(succeeded && error == nil);
    }];
  } @catch (NSException *exception) {
    // No exception text, coordinates, invitation or app hierarchy leaves this helper.
    finish(NO);
  }
}
