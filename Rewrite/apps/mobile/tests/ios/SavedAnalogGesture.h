#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>
#import <XCTest/XCTest.h>
NS_ASSUME_NONNULL_BEGIN
// QA runner only: fixed five-control hold followed by Done, never a product API.
void ThumbleSavedAnalogGesture(XCUIApplication *application, NSArray<NSValue *> *points,
                              CGPoint done, void (^completion)(BOOL succeeded));
NS_ASSUME_NONNULL_END
